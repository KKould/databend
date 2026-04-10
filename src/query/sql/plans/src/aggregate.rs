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

use databend_common_ast::Span;
use databend_common_exception::Result;
use databend_common_expression::Scalar;
use databend_common_expression::types::DataType;
use educe::Educe;

use crate::ColumnSet;
use crate::GenericScalarItem;
use crate::PlanWindowAggregateFunction;
use crate::PlanScalarExpr;
use crate::SortItem;
use crate::Symbol;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub enum AggregateMode {
    Partial,
    Final,

    // TODO(leiysky): this mode is only used for preventing recursion of
    // RuleSplitAggregate, find a better way.
    Initial,
}

/// Information for `GROUPING SETS`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GroupingSets {
    /// The index of the virtual column `_grouping_id`. It's valid only if `grouping_sets` is not empty.
    pub grouping_id_index: Symbol,
    /// See the comment in `GroupingSetsInfo`.
    pub sets: Vec<Vec<Symbol>>,
    /// See the comment in `GroupingSetsInfo`.
    pub dup_group_items: Vec<(Symbol, DataType)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericAggregate<ScalarExpr> {
    pub mode: AggregateMode,
    // group by scalar expressions, such as: group by col1, col2;
    pub group_items: Vec<GenericScalarItem<ScalarExpr>>,
    // aggregate scalar expressions, such as: sum(col1), count(*);
    pub aggregate_functions: Vec<GenericScalarItem<ScalarExpr>>,
    // True if the plan is generated from distinct, else the plan is a normal aggregate;
    pub from_distinct: bool,
    pub rank_limit: Option<(Vec<SortItem>, usize)>,
    pub grouping_sets: Option<GroupingSets>,
}

impl<ScalarExpr> Default for GenericAggregate<ScalarExpr> {
    fn default() -> Self {
        Self {
            mode: AggregateMode::Initial,
            group_items: vec![],
            aggregate_functions: vec![],
            from_distinct: false,
            rank_limit: None,
            grouping_sets: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericAggregateFunctionScalarSortDesc<ScalarExpr> {
    pub expr: ScalarExpr,
    pub is_reuse_index: bool,
    pub nulls_first: bool,
    pub asc: bool,
}

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericAggregateFunction<ScalarExpr> {
    #[educe(PartialEq(ignore), Hash(ignore))]
    pub span: Span,
    pub func_name: String,
    pub distinct: bool,
    pub params: Vec<Scalar>,
    pub args: Vec<ScalarExpr>,
    pub return_type: Box<DataType>,
    pub sort_descs: Vec<GenericAggregateFunctionScalarSortDesc<ScalarExpr>>,
    pub display_name: String,
}

impl<ScalarExpr> GenericAggregateFunction<ScalarExpr> {
    pub fn exprs(&self) -> impl Iterator<Item = &ScalarExpr> {
        self.args
            .iter()
            .chain(self.sort_descs.iter().map(|desc| &desc.expr))
    }

    pub fn exprs_mut(&mut self) -> impl Iterator<Item = &mut ScalarExpr> {
        self.args
            .iter_mut()
            .chain(self.sort_descs.iter_mut().map(|desc| &mut desc.expr))
    }
}

impl<ScalarExpr> PlanWindowAggregateFunction<ScalarExpr> for GenericAggregateFunction<ScalarExpr> {
    fn func_name(&self) -> &str {
        &self.func_name
    }

    fn exprs(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_> {
        Box::new(self.exprs())
    }

    fn return_type(&self) -> DataType {
        (*self.return_type).clone()
    }
}

impl<ScalarExpr> GenericAggregate<ScalarExpr>
where
    ScalarExpr: PlanScalarExpr,
{
    pub fn used_columns(&self) -> Result<ColumnSet> {
        let mut used_columns = ColumnSet::new();
        for group_item in self.group_items.iter() {
            used_columns.insert(group_item.index);
            used_columns.extend(group_item.scalar.used_columns())
        }
        for agg in self.aggregate_functions.iter() {
            used_columns.insert(agg.index);
            used_columns.extend(agg.scalar.used_columns())
        }
        Ok(used_columns)
    }

    pub fn group_columns(&self) -> Result<ColumnSet> {
        let mut col_set = ColumnSet::new();
        for group_item in self.group_items.iter() {
            col_set.insert(group_item.index);
            col_set.extend(group_item.scalar.used_columns())
        }
        Ok(col_set)
    }
}
