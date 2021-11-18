use std::collections::HashMap;
use sqlparser::ast::{Expr, Offset, OrderByExpr, SelectItem, TableWithJoins};
use common_datablocks::DataBlock;
use common_datavalues::DataSchemaRefExt;

use common_exception::{ErrorCode, Result};
use common_planners::{expand_aggregate_arg_exprs, expr_as_column_expr, Expression, extract_aliases, find_aggregate_exprs, find_aggregate_exprs_in_expr, ReadDataSourcePlan, rebase_expr, resolve_aliases_to_exprs};

use crate::sessions::{DatabendQueryContextRef};
use crate::sql::statements::{AnalyzableStatement, AnalyzedResult};
use crate::sql::statements::analyzer_expr::{ExpressionAnalyzer};
use crate::sql::statements::analyzer_statement::QueryAnalyzeState;
use crate::sql::statements::query::{JoinedSchema, JoinedColumnDesc, JoinedSchemaAnalyzer, QualifiedRewriter};
use crate::sql::statements::query::{QueryNormalizerData, QueryNormalizer};

#[derive(Debug, Clone, PartialEq)]
pub struct DfQueryStatement {
    pub from: Vec<TableWithJoins>,
    pub projection: Vec<SelectItem>,
    pub selection: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<Expr>,
    pub offset: Option<Offset>,
}

#[async_trait::async_trait]
impl AnalyzableStatement for DfQueryStatement {
    async fn analyze(&self, ctx: DatabendQueryContextRef) -> Result<AnalyzedResult> {
        let analyzer = JoinedSchemaAnalyzer::create(ctx.clone());
        let joined_schema = analyzer.analyze(self).await?;

        let normal_transform = QueryNormalizer::create(ctx.clone());
        let normalized_result = normal_transform.transform(self).await?;

        let schema = joined_schema.clone();
        let qualified_rewriter = QualifiedRewriter::create(schema, ctx.clone());
        let normalized_result = qualified_rewriter.rewrite(normalized_result).await?;

        let analyze_state = self.analyze_query(normalized_result).await?;
        self.finalize(joined_schema, analyze_state).await
    }
}

impl DfQueryStatement {
    async fn analyze_query(&self, data: QueryNormalizerData) -> Result<QueryAnalyzeState> {
        let mut analyze_state = QueryAnalyzeState::default();

        if let Some(predicate) = &data.filter_predicate {
            Self::verify_no_aggregate(predicate, "filter")?;
            analyze_state.filter = Some(predicate.clone());
        }

        Self::analyze_projection(&data.projection_expressions, &mut analyze_state)?;

        // Allow `SELECT name FROM system.databases HAVING name = 'xxx'`
        if let Some(predicate) = &data.having_predicate {
            // TODO: We can also push having into expressions, which helps:
            //     - SELECT number + 5 AS number FROM numbers(100) HAVING number = 5;
            //     - SELECT number FROM numbers(100) HAVING number + 5 > 5 ORDER BY number + 5 > 5 (bad sql)
            analyze_state.having = Some(predicate.clone());
        }

        for item in &data.order_by_expressions {
            match item {
                Expression::Sort { expr, asc, nulls_first } => {
                    analyze_state.add_expression(&expr);
                    analyze_state.order_by_expressions.push(Expression::Sort {
                        expr: Box::new(rebase_expr(&expr, &analyze_state.expressions)?),
                        asc: *asc,
                        nulls_first: *nulls_first,
                    });
                }
                _ => { return Err(ErrorCode::SyntaxException("Order by must be sort expression. it's a bug.")); }
            }
        }

        if !data.aggregate_expressions.is_empty() || !data.group_by_expressions.is_empty() {
            // Rebase expressions using aggregate expressions and group by expressions
            let mut expressions = Vec::with_capacity(analyze_state.expressions.len());
            for expression in &analyze_state.expressions {
                let expression = rebase_expr(expression, &data.group_by_expressions)?;
                expressions.push(rebase_expr(&expression, &data.aggregate_expressions)?);
            }

            analyze_state.expressions = expressions;

            for group_expression in &data.group_by_expressions {
                analyze_state.add_before_group_expression(group_expression);
                let base_exprs = &analyze_state.before_group_by_expressions;
                analyze_state.group_by_expressions.push(rebase_expr(group_expression, base_exprs)?);
            }

            Self::analyze_aggregate(&data.aggregate_expressions, &mut analyze_state)?;
        }

        Ok(analyze_state)
    }

    fn analyze_aggregate(exprs: &[Expression], state: &mut QueryAnalyzeState) -> Result<()> {
        let aggregate_functions = find_aggregate_exprs(exprs);
        let aggregate_functions_args = expand_aggregate_arg_exprs(&aggregate_functions);

        for aggregate_function_arg in &aggregate_functions_args {
            state.add_before_group_expression(aggregate_function_arg);
        }

        for aggr_expression in exprs {
            let base_exprs = &state.before_group_by_expressions;
            state.aggregate_expressions.push(rebase_expr(aggr_expression, base_exprs)?);
        }

        Ok(())
    }

    fn analyze_projection(exprs: &[Expression], state: &mut QueryAnalyzeState) -> Result<()> {
        for item in exprs {
            match item {
                Expression::Alias(_, expr) => state.add_expression(expr),
                _ => state.add_expression(item),
            }

            let rebased_expr = rebase_expr(item, &state.expressions)?;
            state.projection_expressions.push(rebased_expr);
        }

        Ok(())
    }

    fn verify_no_aggregate(expr: &Expression, info: &str) -> Result<()> {
        match find_aggregate_exprs_in_expr(expr).is_empty() {
            true => Ok(()),
            false => Err(ErrorCode::SyntaxException(format!("{} cannot contain aggregate functions", info))),
        }
    }
}

impl DfQueryStatement {
    pub async fn finalize(&self, schema: JoinedSchema, mut state: QueryAnalyzeState) -> Result<AnalyzedResult> {
        let dry_run_res = Self::verify_with_dry_run(&schema, &state)?;
        state.finalize_schema = dry_run_res.schema().clone();

        // TODO: read source
        Ok(AnalyzedResult::SelectQuery(state))
    }

    fn verify_with_dry_run(schema: &JoinedSchema, state: &QueryAnalyzeState) -> Result<DataBlock> {
        let mut data_block = DataBlock::empty_with_schema(schema.to_data_schema());

        if let Some(predicate) = &state.filter {
            if let Err(cause) = Self::dry_run_expr(predicate, &data_block) {
                return Err(cause.add_message_back(" (while in select filter)"));
            }
        }

        if !state.before_group_by_expressions.is_empty() {
            match Self::dry_run_exprs(&state.before_group_by_expressions, &data_block) {
                Ok(res) => { data_block = res; }
                Err(cause) => {
                    return Err(cause.add_message_back(" (while in select before group by)"));
                }
            }
        }

        if !state.group_by_expressions.is_empty() || !state.aggregate_expressions.is_empty() {
            let new_len = state.aggregate_expressions.len() + state.group_by_expressions.len();
            let mut new_expression = Vec::with_capacity(new_len);

            for group_by_expression in &state.group_by_expressions {
                new_expression.push(group_by_expression);
            }

            for aggregate_expression in &state.aggregate_expressions {
                new_expression.push(aggregate_expression);
            }

            match Self::dry_run_exprs_ref(&new_expression, &data_block) {
                Ok(res) => {
                    data_block = res;
                }
                Err(cause) => {
                    return Err(cause.add_message_back(" (while in select group by)"));
                }
            }
        }

        if !state.expressions.is_empty() {
            match Self::dry_run_exprs(&state.expressions, &data_block) {
                Ok(res) => {
                    data_block = res;
                }
                Err(cause) if state.order_by_expressions.is_empty() => {
                    return Err(cause.add_message_back(" (while in select before projection)"));
                }
                Err(cause) => {
                    return Err(cause.add_message_back(" (while in select before order by)"));
                }
            }
        }

        if let Some(predicate) = &state.having {
            if let Err(cause) = Self::dry_run_expr(predicate, &data_block) {
                return Err(cause.add_message_back(" (while in select having)"));
            }
        }

        if !state.order_by_expressions.is_empty() {
            if let Err(cause) = Self::dry_run_exprs(&state.order_by_expressions, &data_block) {
                return Err(cause.add_message_back(" (while in select order by)"));
            }
        }

        if !state.projection_expressions.is_empty() {
            if let Err(cause) = Self::dry_run_exprs(&state.projection_expressions, &data_block) {
                return Err(cause.add_message_back(" (while in select projection)"));
            }
        }

        Ok(data_block)
    }

    fn dry_run_expr(expr: &Expression, data: &DataBlock) -> Result<DataBlock> {
        let schema = data.schema();
        let data_field = expr.to_data_field(schema)?;
        Ok(DataBlock::empty_with_schema(DataSchemaRefExt::create(vec![data_field])))
    }

    fn dry_run_exprs(exprs: &[Expression], data: &DataBlock) -> Result<DataBlock> {
        let schema = data.schema();
        let mut new_data_fields = Vec::with_capacity(exprs.len());

        for expr in exprs {
            new_data_fields.push(expr.to_data_field(schema)?);
        }

        Ok(DataBlock::empty_with_schema(DataSchemaRefExt::create(new_data_fields)))
    }

    fn dry_run_exprs_ref(exprs: &[&Expression], data: &DataBlock) -> Result<DataBlock> {
        let schema = data.schema();
        let mut new_data_fields = Vec::with_capacity(exprs.len());

        for expr in exprs {
            new_data_fields.push(expr.to_data_field(schema)?);
        }

        Ok(DataBlock::empty_with_schema(DataSchemaRefExt::create(new_data_fields)))
    }
}

