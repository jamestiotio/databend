// Copyright 2020 The FuseQuery Authors.
//
// Code is licensed under AGPL License, Version 3.0.

use crate::datavalues::{
    DataArrayRef, DataType, DataValue, DataValueAggregateOperator, StringArray,
};
use crate::datavalues::{
    Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array,
    UInt32Array, UInt64Array, UInt8Array,
};
use crate::error::{FuseQueryError, FuseQueryResult};

pub fn data_array_aggregate_op(
    op: DataValueAggregateOperator,
    value: DataArrayRef,
) -> FuseQueryResult<DataValue> {
    Ok(match value.data_type() {
        DataType::Int8 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Int8Array, Int8, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Int8Array, Int8, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Int8Array, Int8)
            }
        },
        DataType::Int16 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Int16Array, Int16, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Int16Array, Int16, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Int16Array, Int16)
            }
        },
        DataType::Int32 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Int32Array, Int32, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Int32Array, Int32, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Int32Array, Int32)
            }
        },
        DataType::Int64 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Int64Array, Int64, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Int64Array, Int64, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Int64Array, Int64)
            }
        },
        DataType::UInt8 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, UInt8Array, UInt8, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, UInt8Array, UInt8, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, UInt8Array, UInt8)
            }
        },
        DataType::UInt16 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, UInt16Array, UInt16, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, UInt16Array, UInt16, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, UInt16Array, UInt16)
            }
        },
        DataType::UInt32 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, UInt32Array, UInt32, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, UInt32Array, UInt32, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, UInt32Array, UInt32)
            }
        },
        DataType::UInt64 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, UInt64Array, UInt64, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, UInt64Array, UInt64, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, UInt64Array, UInt64)
            }
        },
        DataType::Float32 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Float32Array, Float32, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Float32Array, Float32, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Float32Array, Float32)
            }
        },
        DataType::Float64 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_to_data_value!(value, Float64Array, Float64, min)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_to_data_value!(value, Float64Array, Float64, max)
            }
            DataValueAggregateOperator::Sum => {
                typed_array_sum_to_data_value!(value, Float64Array, Float64)
            }
        },
        DataType::Utf8 => match op {
            DataValueAggregateOperator::Min => {
                typed_array_min_max_string_to_data_value!(value, StringArray, String, min_string)
            }
            DataValueAggregateOperator::Max => {
                typed_array_min_max_string_to_data_value!(value, StringArray, String, max_string)
            }
            _ => {
                return Err(FuseQueryError::Internal(format!(
                    "Unsupported data_array_{} for data type: {:?}",
                    op,
                    value.data_type()
                )))
            }
        },
        _ => {
            return Err(FuseQueryError::Internal(format!(
                "Unsupported data_array_{} for data type: {:?}",
                op,
                value.data_type()
            )))
        }
    })
}
