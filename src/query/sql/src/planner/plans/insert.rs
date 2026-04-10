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

use super::Plan;
use crate::planner::format::FormatOptions;
use crate::planner::format::MetadataIdHumanizer;
use crate::plans::CopyIntoTablePlan;

pub use databend_common_sql_plans::InsertValue;
pub use databend_common_sql_plans::StreamingLoadPlan;

pub type Insert = databend_common_sql_plans::GenericInsert<Plan>;
pub type InsertInputSource = databend_common_sql_plans::GenericInsertInputSource<Plan>;

pub fn format_insert_source(
    plan_name: &str,
    source: &InsertInputSource,
    options: FormatOptions,
    mut children: Vec<FormatTreeNode>,
) -> databend_common_exception::Result<String> {
    match source {
        InsertInputSource::SelectPlan(plan) => {
            if let Plan::Query {
                s_expr, metadata, ..
            } = &**plan
            {
                let metadata = &*metadata.read();
                let humanizer = MetadataIdHumanizer::new(metadata, options);
                let sub_tree = s_expr.to_format_tree(&humanizer)?;
                children.push(sub_tree);

                return Ok(FormatTreeNode::with_children(
                    format!("{plan_name} (subquery):"),
                    children,
                )
                .format_pretty()?);
            }
            Ok(String::new())
        }
        InsertInputSource::Values(values) => match values {
            InsertValue::Values { .. } => Ok(FormatTreeNode::with_children(
                format!("{plan_name} (values):"),
                children,
            )
            .format_pretty()?),
            InsertValue::RawValues { .. } => Ok(FormatTreeNode::with_children(
                format!("{plan_name} (rawvalues):"),
                children,
            )
            .format_pretty()?),
        },
        InsertInputSource::StreamingLoad(plan) => {
            let stage_node = vec![FormatTreeNode::new(format!("format: {}", plan.file_format))];
            children.extend(stage_node);

            Ok(FormatTreeNode::with_children(
                "InsertPlan (StreamingWithFileFormat):".to_string(),
                children,
            )
            .format_pretty()?)
        }
        InsertInputSource::Stage(plan) => match *plan.clone() {
            Plan::CopyIntoTable(copy_plan) => {
                let CopyIntoTablePlan {
                    no_file_to_copy,
                    from_attachment,
                    required_values_schema,
                    required_source_schema,
                    write_mode,
                    validation_mode,
                    stage_table_info,
                    enable_distributed,
                    ..
                } = &*copy_plan;
                let required_values_schema = required_values_schema
                    .fields()
                    .iter()
                    .map(|field| field.name().to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let required_source_schema = required_source_schema
                    .fields()
                    .iter()
                    .map(|field| field.name().to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let stage_node = vec![
                    FormatTreeNode::new(format!("no_file_to_copy: {no_file_to_copy}")),
                    FormatTreeNode::new(format!("from_attachment: {from_attachment}")),
                    FormatTreeNode::new(format!(
                        "required_values_schema: [{required_values_schema}]"
                    )),
                    FormatTreeNode::new(format!(
                        "required_source_schema: [{required_source_schema}]"
                    )),
                    FormatTreeNode::new(format!("write_mode: {write_mode}")),
                    FormatTreeNode::new(format!("validation_mode: {validation_mode}")),
                    FormatTreeNode::new(format!("stage_table_info: {stage_table_info}")),
                    FormatTreeNode::new(format!("enable_distributed: {enable_distributed}")),
                ];
                children.extend(stage_node);
                Ok(
                    FormatTreeNode::with_children(format!("{plan_name} (stage):"), children)
                        .format_pretty()?,
                )
            }
            _ => unreachable!("plan in InsertInputSource::Stag must be CopyIntoTable"),
        },
    }
}

#[async_backtrace::framed]
pub async fn explain_insert(
    plan: &Insert,
    options: FormatOptions,
) -> databend_common_exception::Result<Vec<DataBlock>> {
    let mut result = vec![];

    let Insert {
        catalog,
        database,
        table,
        branch,
        schema,
        overwrite,
        table_info: _,
        source,
    } = plan;

    let table_name = if let Some(branch) = branch {
        format!("{}.{}.{}/{}", catalog, database, table, branch)
    } else {
        format!("{}.{}.{}", catalog, database, table)
    };
    let inserted_columns = schema
        .fields
        .iter()
        .map(|field| format!("{}.{} (#{})", table, field.name, field.column_id))
        .collect::<Vec<_>>()
        .join(",");

    let children = vec![
        FormatTreeNode::new(format!("table: {table_name}")),
        FormatTreeNode::new(format!("inserted columns: [{inserted_columns}]")),
        FormatTreeNode::new(format!("overwrite: {overwrite}")),
    ];

    let formatted_plan = format_insert_source("InsertPlan", source, options, children)?;
    let line_split_result: Vec<&str> = formatted_plan.lines().collect();
    let formatted_plan = StringType::from_data(line_split_result);
    result.push(DataBlock::new_from_columns(vec![formatted_plan]));
    Ok(vec![DataBlock::concat(&result)?])
}
