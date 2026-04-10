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

use databend_common_ast::ast::Expr;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::TableField;
use databend_common_expression::TableSchemaRef;
use databend_common_pipeline::core::SharedLockGuard;
use databend_meta_client::types::MetaId;

use crate::GenericInsertInputSource;

#[derive(Clone)]
pub struct GenericReplace<QueryPlan> {
    pub catalog: String,
    pub database: String,
    pub table: String,
    pub table_id: MetaId,
    pub on_conflict_fields: Vec<TableField>,
    pub schema: TableSchemaRef,
    pub source: GenericInsertInputSource<QueryPlan>,
    pub delete_when: Option<Expr>,
    pub lock_guard: Option<SharedLockGuard>,
}

impl<QueryPlan> PartialEq for GenericReplace<QueryPlan> {
    fn eq(&self, other: &Self) -> bool {
        self.catalog == other.catalog
            && self.database == other.database
            && self.table == other.table
            && self.schema == other.schema
            && self.on_conflict_fields == other.on_conflict_fields
    }
}

impl<QueryPlan> GenericReplace<QueryPlan> {
    pub fn schema(&self) -> DataSchemaRef {
        Arc::new(self.schema.clone().into())
    }

    pub fn has_select_plan(&self) -> bool {
        matches!(&self.source, GenericInsertInputSource::SelectPlan(_))
    }
}

impl<QueryPlan> std::fmt::Debug for GenericReplace<QueryPlan> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Replace")
            .field("catalog", &self.catalog)
            .field("database", &self.database)
            .field("table", &self.table)
            .field("table_id", &self.table_id)
            .field("schema", &self.schema)
            .field("on conflict", &self.on_conflict_fields)
            .finish()
    }
}
