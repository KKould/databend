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

use databend_common_ast::ast::FormatTreeNode;
use databend_common_expression::DataBlock;
use databend_common_expression::FromData;
use databend_common_expression::types::StringType;

use super::insert::format_insert_source;
use crate::FormatOptions;

pub type Replace = databend_common_sql_plans::GenericReplace<super::Plan>;

#[async_backtrace::framed]
pub async fn explain_replace(
    plan: &Replace,
    options: FormatOptions,
) -> databend_common_exception::Result<Vec<DataBlock>> {
    let mut result = vec![];

    let Replace {
        catalog,
        database,
        table,
        source,
        on_conflict_fields,
        ..
    } = plan;

    let table_name = format!("{}.{}.{}", catalog, database, table);
    let on_columns = on_conflict_fields
        .iter()
        .map(|field| format!("{}.{} (#{})", table, field.name, field.column_id))
        .collect::<Vec<_>>()
        .join(",");

    let children = vec![
        FormatTreeNode::new(format!("table: {table_name}")),
        FormatTreeNode::new(format!("on columns: [{on_columns}]")),
    ];

    let formatted_plan = format_insert_source("ReplacePlan", source, options, children)?;
    let line_split_result: Vec<&str> = formatted_plan.lines().collect();
    let formatted_plan = StringType::from_data(line_split_result);
    result.push(DataBlock::new_from_columns(vec![formatted_plan]));
    Ok(vec![DataBlock::concat(&result)?])
}
