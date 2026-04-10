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

use crate::GenericScalarItem;
use crate::Symbol;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SortItem {
    pub index: Symbol,
    pub asc: bool,
    pub nulls_first: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericWindowOrderByInfo<ScalarExpr> {
    pub order_by_item: GenericScalarItem<ScalarExpr>,
    pub asc: Option<bool>,
    pub nulls_first: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericWindowPartition<WindowFuncType, ScalarExpr> {
    pub partition_by: Vec<GenericScalarItem<ScalarExpr>>,
    pub top: Option<usize>,
    pub func: WindowFuncType,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericSort<WindowPartition> {
    pub items: Vec<SortItem>,
    pub limit: Option<usize>,
    pub after_exchange: Option<bool>,
    pub pre_projection: Option<Vec<Symbol>>,
    pub window_partition: Option<WindowPartition>,
}
