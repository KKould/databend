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

use std::sync::Arc;

use databend_common_exception::Result;
use databend_common_expression::BlockThresholds;
use databend_common_expression::DataBlock;
use databend_common_expression::DataField;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::DataSchemaRefExt;
use databend_common_expression::RemoteDefaultExpr;
use databend_common_expression::Scalar;
use databend_common_expression::TableSchemaRef;
use databend_common_expression::types::DataType;
use databend_common_expression::types::NumberDataType;
use databend_common_meta_app::principal::FileFormatParams;
use databend_common_meta_app::schema::TableInfo;
use enum_as_inner::EnumAsInner;
use parking_lot::Mutex;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc::Receiver;

const INSERT_NAME: &str = "number of rows inserted";

#[derive(Clone, Debug, EnumAsInner)]
pub enum GenericInsertInputSource<QueryPlan> {
    SelectPlan(Box<QueryPlan>),
    Values(InsertValue),
    Stage(Box<QueryPlan>),
    StreamingLoad(StreamingLoadPlan),
}

#[derive(Clone, Debug)]
pub struct StreamingLoadPlan {
    pub file_format: Box<FileFormatParams>,
    pub required_values_schema: DataSchemaRef,
    pub values_consts: Vec<Scalar>,
    pub required_source_schema: TableSchemaRef,
    pub default_exprs: Option<Vec<RemoteDefaultExpr>>,
    pub block_thresholds: BlockThresholds,
    pub receiver: Arc<Mutex<Option<Receiver<Result<DataBlock>>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsertValue {
    Values { rows: Vec<Vec<Scalar>> },
    RawValues { data: String, start: usize },
}

#[derive(Clone)]
pub struct GenericInsert<QueryPlan> {
    pub catalog: String,
    pub database: String,
    pub table: String,
    pub branch: Option<String>,
    pub schema: TableSchemaRef,
    pub overwrite: bool,
    pub source: GenericInsertInputSource<QueryPlan>,
    pub table_info: Option<TableInfo>,
}

impl<QueryPlan> PartialEq for GenericInsert<QueryPlan> {
    fn eq(&self, other: &Self) -> bool {
        self.catalog == other.catalog
            && self.database == other.database
            && self.table == other.table
            && self.schema == other.schema
            && self.branch == other.branch
    }
}

impl<QueryPlan> GenericInsert<QueryPlan> {
    pub fn dest_schema(&self) -> DataSchemaRef {
        Arc::new(self.schema.clone().into())
    }

    pub fn has_select_plan(&self) -> bool {
        matches!(&self.source, GenericInsertInputSource::SelectPlan(_))
    }

    pub fn schema(&self) -> DataSchemaRef {
        DataSchemaRefExt::create(vec![DataField::new(
            INSERT_NAME,
            DataType::Number(NumberDataType::UInt64),
        )])
    }
}

impl<QueryPlan> std::fmt::Debug for GenericInsert<QueryPlan> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Insert")
            .field("catalog", &self.catalog)
            .field("database", &self.database)
            .field("table", &self.table)
            .field("branch", &self.branch)
            .field("schema", &self.schema)
            .field("overwrite", &self.overwrite)
            .finish()
    }
}
