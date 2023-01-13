//  Copyright 2022 Datafuse Labs.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use std::sync::Arc;

use common_catalog::table_context::TableContext;
use common_exception::Result;
use common_expression::Expr;
use common_expression::TableSchemaRef;
use storages_common_index::RangeFilter;
use storages_common_table_meta::meta::StatisticsOfColumns;

pub trait RangePruner {
    // returns ture, if target should NOT be pruned (false positive allowed)
    fn should_keep(&self, input: &StatisticsOfColumns) -> bool;
}

struct KeepTrue;

impl RangePruner for KeepTrue {
    fn should_keep(&self, _input: &StatisticsOfColumns) -> bool {
        true
    }
}

struct KeepFalse;

impl RangePruner for KeepFalse {
    fn should_keep(&self, _input: &StatisticsOfColumns) -> bool {
        false
    }
}

impl RangePruner for RangeFilter {
    fn should_keep(&self, stats: &StatisticsOfColumns) -> bool {
        match self.eval(stats) {
            Ok(r) => r,
            Err(e) => {
                // swallow exceptions intentionally, corrupted index should not prevent execution
                tracing::warn!("failed to range filter, returning ture. {}", e);
                true
            }
        }
    }
}

pub struct RangePrunerCreator;

impl RangePrunerCreator {
    /// Create a new [`RangePruner`] from expression and schema.
    ///
    /// Note: the schema should be the schema of the table, not the schema of the input.
    pub fn try_create<'a>(
        ctx: &Arc<dyn TableContext>,
        filter_expr: Option<&'a [Expr<String>]>,
        schema: &'a TableSchemaRef,
    ) -> Result<Arc<dyn RangePruner + Send + Sync>> {
        Ok(match filter_expr {
            Some(exprs) if !exprs.is_empty() => {
                let range_filter = RangeFilter::try_create(ctx.clone(), exprs, schema.clone())?;
                match range_filter.try_eval_const() {
                    Ok(v) => {
                        if v {
                            Arc::new(range_filter)
                        } else {
                            Arc::new(KeepFalse)
                        }
                    }
                    Err(_) => Arc::new(range_filter),
                }
            }
            _ => Arc::new(KeepTrue),
        })
    }
}