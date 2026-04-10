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

use databend_common_catalog::table_context::TableContext;
use databend_common_exception::Result;
use databend_common_expression::types::DataType;

use super::AggregateFunction;
use super::NthValueFunction;
use crate::ColumnSet;
use crate::ScalarExpr;
use crate::optimizer::ir::Distribution;
use crate::optimizer::ir::RelExpr;
use crate::optimizer::ir::RelationalProperty;
use crate::optimizer::ir::RequiredProperty;
use crate::optimizer::ir::StatInfo;
use crate::plans::LagLeadFunction;
use crate::plans::NtileFunction;
use crate::plans::Operator;
use crate::plans::RelOp;
pub type Window = databend_common_sql_plans::GenericWindow<WindowFuncType, ScalarExpr>;
pub use databend_common_sql_plans::WindowFuncFrame;
pub use databend_common_sql_plans::WindowFuncFrameBound;
pub use databend_common_sql_plans::WindowFuncFrameUnits;
pub type WindowFuncType = databend_common_sql_plans::GenericWindowFuncType<
    AggregateFunction,
    LagLeadFunction,
    NthValueFunction,
    NtileFunction,
>;

pub trait WindowFuncTypeExt {
    fn func_name(&self) -> String;
    fn used_columns(&self) -> ColumnSet;
    fn return_type(&self) -> DataType;
}

impl WindowFuncTypeExt for WindowFuncType {
    fn func_name(&self) -> String {
        <Self as databend_common_sql_plans::PlanWindowFunction<ScalarExpr>>::func_name(self)
    }

    fn used_columns(&self) -> ColumnSet {
        <Self as databend_common_sql_plans::PlanWindowFunction<ScalarExpr>>::used_columns(self)
    }

    fn return_type(&self) -> DataType {
        <Self as databend_common_sql_plans::PlanWindowFunction<ScalarExpr>>::return_type(self)
    }
}

impl Operator for Window {
    fn rel_op(&self) -> RelOp {
        RelOp::Window
    }

    fn scalar_expr_iter(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_> {
        databend_common_sql_plans::GenericWindow::<WindowFuncType, ScalarExpr>::scalar_expr_iter(
            self,
        )
    }

    fn compute_required_prop_child(
        &self,
        _ctx: Arc<dyn TableContext>,
        _rel_expr: &RelExpr,
        _child_index: usize,
        required: &RequiredProperty,
    ) -> Result<RequiredProperty> {
        let mut required = required.clone();
        if self.partition_by.is_empty() {
            required.distribution = Distribution::Serial;
        }
        Ok(required.clone())
    }

    fn compute_required_prop_children(
        &self,
        _ctx: Arc<dyn TableContext>,
        _rel_expr: &RelExpr,
        required: &RequiredProperty,
    ) -> Result<Vec<Vec<RequiredProperty>>> {
        let mut required = required.clone();
        if self.partition_by.is_empty() {
            required.distribution = Distribution::Serial;
        }
        Ok(vec![vec![required.clone()]])
    }

    fn derive_relational_prop(&self, rel_expr: &RelExpr) -> Result<Arc<RelationalProperty>> {
        let input_prop = rel_expr.derive_relational_prop_child(0)?;

        // Derive output columns
        let mut output_columns = input_prop.output_columns.clone();
        output_columns.insert(self.index);

        // Derive outer columns
        let outer_columns = input_prop
            .outer_columns
            .difference(&output_columns)
            .cloned()
            .collect();

        // Derive used columns
        let mut used_columns =
            databend_common_sql_plans::GenericWindow::<WindowFuncType, ScalarExpr>::used_columns(
                self,
            )?;
        used_columns.extend(input_prop.used_columns.clone());

        // Derive orderings
        let orderings = input_prop.orderings.clone();
        let partition_orderings = input_prop.partition_orderings.clone();

        Ok(Arc::new(RelationalProperty {
            output_columns,
            outer_columns,
            used_columns,
            orderings,
            partition_orderings,
        }))
    }

    fn derive_stats(&self, rel_expr: &RelExpr) -> Result<Arc<StatInfo>> {
        rel_expr.derive_cardinality_child(0)
    }
}

pub type WindowOrderByInfo = databend_common_sql_plans::GenericWindowOrderByInfo<ScalarExpr>;
pub type WindowPartition =
    databend_common_sql_plans::GenericWindowPartition<WindowFuncType, ScalarExpr>;
