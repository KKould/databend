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

use databend_common_expression::DataField;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::DataSchemaRefExt;
use databend_common_expression::types::DataType;
use databend_common_expression::types::NumberDataType;

#[derive(Clone, Debug)]
pub struct GenericInsertMultiTable<QueryPlan, ScalarExpr, Metadata> {
    pub overwrite: bool,
    pub is_first: bool,
    pub input_source: QueryPlan,
    pub whens: Vec<When<ScalarExpr>>,
    pub opt_else: Option<Else<ScalarExpr>>,
    pub intos: Vec<Into<ScalarExpr>>,
    pub target_tables: Vec<(u64, (String, String))>,
    pub meta_data: Metadata,
}

#[derive(Clone, Debug)]
pub struct When<ScalarExpr> {
    pub condition: ScalarExpr,
    pub intos: Vec<Into<ScalarExpr>>,
}

#[derive(Clone, Debug)]
pub struct Into<ScalarExpr> {
    pub catalog: String,
    pub database: String,
    pub table: String,
    pub source_scalar_exprs: Option<Vec<ScalarExpr>>,
    pub casted_schema: DataSchemaRef,
}

#[derive(Clone, Debug)]
pub struct Else<ScalarExpr> {
    pub intos: Vec<Into<ScalarExpr>>,
}

impl<QueryPlan, ScalarExpr, Metadata> GenericInsertMultiTable<QueryPlan, ScalarExpr, Metadata> {
    pub fn schema(&self) -> DataSchemaRef {
        let mut fields = vec![];
        for (_, (db, tbl)) in self.target_tables.iter() {
            let field_name = format!("number of rows inserted into {}.{}", db, tbl);
            fields.push(DataField::new(
                &field_name,
                DataType::Number(NumberDataType::UInt64),
            ));
        }
        DataSchemaRefExt::create(fields)
    }
}
