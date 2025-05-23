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

use databend_common_exception::Result;
use databend_common_expression::ConstantFolder;
use databend_common_expression::DataField;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::DataSchemaRefExt;
use databend_common_expression::RemoteExpr;
use databend_common_functions::BUILTIN_FUNCTIONS;

use crate::executor::explain::PlanStatsInfo;
use crate::executor::PhysicalPlan;
use crate::executor::PhysicalPlanBuilder;
use crate::optimizer::ir::SExpr;
use crate::ColumnSet;
use crate::IndexType;
use crate::TypeCheck;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectSet {
    // A unique id of operator in a `PhysicalPlan` tree, only used for display.
    pub plan_id: u32,
    pub projections: ColumnSet,
    pub input: Box<PhysicalPlan>,
    pub srf_exprs: Vec<(RemoteExpr, IndexType)>,

    // Only used for explain
    pub stat_info: Option<PlanStatsInfo>,
}

impl ProjectSet {
    pub fn output_schema(&self) -> Result<DataSchemaRef> {
        let input_schema = self.input.output_schema()?;
        let mut fields = Vec::with_capacity(input_schema.num_fields() + self.srf_exprs.len());
        for (i, field) in input_schema.fields().iter().enumerate() {
            if self.projections.contains(&i) {
                fields.push(field.clone());
            }
        }
        fields.extend(self.srf_exprs.iter().map(|(srf, index)| {
            DataField::new(
                &index.to_string(),
                srf.as_expr(&BUILTIN_FUNCTIONS).data_type().clone(),
            )
        }));
        Ok(DataSchemaRefExt::create(fields))
    }
}

impl PhysicalPlanBuilder {
    pub(crate) async fn build_project_set(
        &mut self,
        s_expr: &SExpr,
        project_set: &crate::plans::ProjectSet,
        mut required: ColumnSet,
        stat_info: PlanStatsInfo,
    ) -> Result<PhysicalPlan> {
        // 1. Prune unused Columns.
        let column_projections = required.clone().into_iter().collect::<Vec<_>>();
        for s in project_set.srfs.iter() {
            required.extend(s.scalar.used_columns().iter().copied());
        }

        // 2. Build physical plan.
        let input = self.build(s_expr.child(0)?, required).await?;
        let input_schema = input.output_schema()?;
        let srf_exprs = project_set
            .srfs
            .iter()
            .map(|item| {
                let expr = item
                    .scalar
                    .type_check(input_schema.as_ref())?
                    .project_column_ref(|index| input_schema.index_of(&index.to_string()).unwrap());
                let (expr, _) = ConstantFolder::fold(&expr, &self.func_ctx, &BUILTIN_FUNCTIONS);
                Ok((expr.as_remote_expr(), item.index))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut projections = ColumnSet::new();
        for column in column_projections.iter() {
            if let Some((index, _)) = input_schema.column_with_name(&column.to_string()) {
                projections.insert(index);
            }
        }

        Ok(PhysicalPlan::ProjectSet(ProjectSet {
            plan_id: 0,
            input: Box::new(input),
            srf_exprs,
            projections,
            stat_info: Some(stat_info),
        }))
    }
}
