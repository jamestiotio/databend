// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use common_datavalues::prelude::*;
use common_exception::Result;
use common_planners::col;
use common_planners::lit;
use pretty_assertions::assert_eq;

use crate::PartitionIndex;

#[test]
fn test_partition_index() -> Result<()> {
    // Apply index.
    {
        let partition_value = DataValue::String(Some("datafuse".as_bytes().to_vec()));
        let expr = col("name").eq(lit("bohu".as_bytes()));
        let actual = PartitionIndex::apply_index(partition_value, &expr)?;
        let expected = true;
        assert_eq!(actual, expected);
    }

    Ok(())
}
