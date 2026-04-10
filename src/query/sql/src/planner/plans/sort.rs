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

use super::WindowPartition;
use crate::ColumnSet;
use crate::optimizer::ir::Distribution;
use crate::optimizer::ir::PhysicalProperty;
use crate::optimizer::ir::RelExpr;
use crate::optimizer::ir::RelationalProperty;
use crate::optimizer::ir::RequiredProperty;
use crate::optimizer::ir::StatInfo;
use crate::plans::Operator;
use crate::plans::RelOp;

pub type Sort = databend_common_sql_plans::GenericSort<WindowPartition>;
pub use databend_common_sql_plans::SortItem;

pub trait SortExt {
    fn used_columns(&self) -> ColumnSet;

    fn sort_items_exclude_partition(&self) -> Vec<SortItem>;

    fn replace_column(
        &mut self,
        old: databend_common_sql_plans::Symbol,
        new: databend_common_sql_plans::Symbol,
    );
}

impl SortExt for Sort {
    fn used_columns(&self) -> ColumnSet {
        self.items.iter().map(|item| item.index).collect()
    }

    fn sort_items_exclude_partition(&self) -> Vec<SortItem> {
        self.items
            .iter()
            .filter(|item| match &self.window_partition {
                Some(window) => !window
                    .partition_by
                    .iter()
                    .any(|partition| partition.index == item.index),
                None => true,
            })
            .cloned()
            .collect()
    }

    fn replace_column(
        &mut self,
        old: databend_common_sql_plans::Symbol,
        new: databend_common_sql_plans::Symbol,
    ) {
        for item in &mut self.items {
            if item.index == old {
                item.index = new
            }
        }

        if let Some(projection) = &mut self.pre_projection {
            for i in projection {
                if *i == old {
                    *i = new
                }
            }
        }

        if self.window_partition.is_some() {
            unimplemented!()
        };
    }
}

impl Operator for Sort {
    fn rel_op(&self) -> RelOp {
        RelOp::Sort
    }

    fn derive_physical_prop(&self, rel_expr: &RelExpr) -> Result<PhysicalProperty> {
        let input_physical_prop = rel_expr.derive_physical_prop_child(0)?;
        if input_physical_prop.distribution == Distribution::Serial {
            return Ok(input_physical_prop);
        }
        let Some(window) = &self.window_partition else {
            return Ok(input_physical_prop);
        };

        let partition_by = window
            .partition_by
            .iter()
            .map(|s| s.scalar.clone())
            .collect();
        Ok(PhysicalProperty {
            distribution: Distribution::GlobalHash(partition_by),
        })
    }

    fn compute_required_prop_child(
        &self,
        _ctx: Arc<dyn TableContext>,
        rel_expr: &RelExpr,
        _child_index: usize,
        required: &RequiredProperty,
    ) -> Result<RequiredProperty> {
        let mut required = required.clone();
        required.distribution = Distribution::Serial;

        let Some(window) = &self.window_partition else {
            return Ok(required);
        };

        let child_physical_prop = rel_expr.derive_physical_prop_child(0)?;
        // Can't merge to shuffle
        if child_physical_prop.distribution == Distribution::Serial {
            return Ok(required);
        }

        let partition_by = window
            .partition_by
            .iter()
            .map(|s| s.scalar.clone())
            .collect();
        required.distribution = Distribution::GlobalHash(partition_by);

        Ok(required)
    }

    fn compute_required_prop_children(
        &self,
        _ctx: Arc<dyn TableContext>,
        rel_expr: &RelExpr,
        required: &RequiredProperty,
    ) -> Result<Vec<Vec<RequiredProperty>>> {
        let mut required = required.clone();
        required.distribution = Distribution::Serial;

        let Some(window) = &self.window_partition else {
            return Ok(vec![vec![required]]);
        };

        // Can't merge to shuffle
        let child_physical_prop = rel_expr.derive_physical_prop_child(0)?;
        if child_physical_prop.distribution == Distribution::Serial {
            return Ok(vec![vec![required]]);
        }

        let partition_by = window
            .partition_by
            .iter()
            .map(|s| s.scalar.clone())
            .collect();

        required.distribution = Distribution::GlobalHash(partition_by);
        Ok(vec![vec![required]])
    }

    fn derive_relational_prop(&self, rel_expr: &RelExpr) -> Result<Arc<RelationalProperty>> {
        let input_prop = rel_expr.derive_relational_prop_child(0)?;

        let output_columns = input_prop.output_columns.clone();
        let outer_columns = input_prop.outer_columns.clone();
        let used_columns = input_prop.used_columns.clone();

        // Derive orderings
        let orderings = self.items.clone();

        let (orderings, partition_orderings) = match &self.window_partition {
            Some(window) => (
                vec![],
                Some((window.partition_by.clone(), orderings.clone())),
            ),
            None => (self.items.clone(), None),
        };

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
