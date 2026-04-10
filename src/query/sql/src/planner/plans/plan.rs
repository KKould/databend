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

use databend_common_expression::DataSchemaRef;

use crate::BindContext;
use crate::MetadataRef;
use crate::ScalarExpr;
use crate::binder::ExplainConfig;
use crate::optimizer::ir::SExpr;
use crate::plans::Exchange;
use crate::plans::RelOperator;
use crate::plans::ReclusterPlan;

pub use databend_common_sql_plans::RewriteKind;

pub type Plan = databend_common_sql_plans::GenericPlan<
    SExpr,
    MetadataRef,
    BindContext,
    ExplainConfig,
    RewriteKind,
    ScalarExpr,
    ReclusterPlan,
>;

impl databend_common_sql_plans::PlanQueryBindContext for BindContext {
    fn output_schema(&self) -> DataSchemaRef {
        self.output_schema()
    }
}

impl databend_common_sql_plans::PlanQuerySExpr for SExpr {
    fn has_merge_exchange(&self) -> bool {
        matches!(self.plan.as_ref(), RelOperator::Exchange(Exchange::Merge))
    }

    fn first_child(&self) -> Option<Self> {
        self.child(0).ok().cloned()
    }
}
