// Copyright 2021 Datafuse Labs
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

use std::fmt::Debug;
use std::fmt::Formatter;

use databend_common_expression::DataField;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::DataSchemaRefExt;
use databend_common_expression::RemoteExpr;
use databend_common_expression::types::DataType;
use databend_common_expression::types::NumberDataType;
use databend_storages_common_stage::CopyIntoLocationInfo;

#[derive(Clone)]
pub struct GenericCopyIntoLocationPlan<QueryPlan, ScalarExpr> {
    pub info: CopyIntoLocationInfo,
    pub from: Box<QueryPlan>,
    pub partition_by: Option<GenericPartitionByDesc<ScalarExpr>>,
}

#[derive(Clone, Debug)]
pub struct GenericPartitionByDesc<ScalarExpr> {
    pub display: String,
    pub expr: ScalarExpr,
    pub remote_expr: RemoteExpr,
    pub nullable: bool,
}

impl<QueryPlan, ScalarExpr> GenericCopyIntoLocationPlan<QueryPlan, ScalarExpr> {
    pub fn schema(&self) -> DataSchemaRef {
        if self.info.options.detailed_output {
            DataSchemaRefExt::create(vec![
                DataField::new("file_name", DataType::String),
                DataField::new("file_size", DataType::Number(NumberDataType::UInt64)),
                DataField::new("row_count", DataType::Number(NumberDataType::UInt64)),
            ])
        } else {
            DataSchemaRefExt::create(vec![
                DataField::new("rows_unloaded", DataType::Number(NumberDataType::UInt64)),
                DataField::new("input_bytes", DataType::Number(NumberDataType::UInt64)),
                DataField::new("output_bytes", DataType::Number(NumberDataType::UInt64)),
            ])
        }
    }
}

impl<QueryPlan: Debug, ScalarExpr> Debug for GenericCopyIntoLocationPlan<QueryPlan, ScalarExpr> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Copy into {:?}/{} from {:?}",
            self.info.stage, self.info.path, self.from
        )?;
        if let Some(partition_by) = &self.partition_by {
            write!(f, " partition_by={}", partition_by.display)?;
        }
        Ok(())
    }
}
