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

use crate::ColumnSet;
use crate::IndexType;
use crate::PlanScalarExpr;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericSecureFilter<ScalarExpr> {
    pub predicates: Vec<ScalarExpr>,
    pub table_index: IndexType,
}

impl<ScalarExpr> GenericSecureFilter<ScalarExpr>
where ScalarExpr: PlanScalarExpr
{
    pub fn used_columns(&self) -> Result<ColumnSet> {
        Ok(self
            .predicates
            .iter()
            .map(PlanScalarExpr::used_columns)
            .fold(ColumnSet::new(), |acc, x| acc.union(&x).cloned().collect()))
    }
}
