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

use std::fmt::Display;
use std::fmt::Formatter;

use databend_common_exception::Result;

use crate::ColumnSet;
use crate::PlanScalarExpr;
use crate::Symbol;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum JoinType {
    Cross,
    Inner,
    InnerAny,
    Left,
    LeftAny,
    Right,
    RightAny,
    Full,
    LeftSemi,
    RightSemi,
    LeftAnti,
    RightAnti,
    LeftMark,
    RightMark,
    LeftSingle,
    RightSingle,
    Asof,
    LeftAsof,
    RightAsof,
}

impl JoinType {
    pub fn opposite(&self) -> JoinType {
        match self {
            JoinType::Left => JoinType::Right,
            JoinType::LeftAny => JoinType::RightAny,
            JoinType::Right => JoinType::Left,
            JoinType::RightAny => JoinType::LeftAny,
            JoinType::LeftSingle => JoinType::RightSingle,
            JoinType::RightSingle => JoinType::LeftSingle,
            JoinType::LeftSemi => JoinType::RightSemi,
            JoinType::RightSemi => JoinType::LeftSemi,
            JoinType::LeftAnti => JoinType::RightAnti,
            JoinType::RightAnti => JoinType::LeftAnti,
            JoinType::LeftMark => JoinType::RightMark,
            JoinType::RightMark => JoinType::LeftMark,
            JoinType::RightAsof => JoinType::LeftAsof,
            JoinType::LeftAsof => JoinType::RightAsof,
            _ => *self,
        }
    }

    pub fn is_outer_join(&self) -> bool {
        matches!(
            self,
            JoinType::Left
                | JoinType::LeftAny
                | JoinType::Right
                | JoinType::RightAny
                | JoinType::Full
                | JoinType::LeftSingle
                | JoinType::RightSingle
                | JoinType::LeftAsof
                | JoinType::RightAsof
        )
    }

    pub fn is_mark_join(&self) -> bool {
        matches!(self, JoinType::LeftMark | JoinType::RightMark)
    }

    pub fn is_any_join(&self) -> bool {
        matches!(
            self,
            JoinType::InnerAny | JoinType::LeftAny | JoinType::RightAny
        )
    }

    pub fn is_asof_join(&self) -> bool {
        matches!(
            self,
            JoinType::Asof | JoinType::LeftAsof | JoinType::RightAsof
        )
    }

    pub fn is_filtering_join(&self) -> bool {
        matches!(
            self,
            JoinType::Inner
                | JoinType::InnerAny
                | JoinType::LeftSemi
                | JoinType::RightSemi
                | JoinType::LeftAnti
                | JoinType::RightAnti
        )
    }
}

impl Display for JoinType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER"),
            JoinType::InnerAny => write!(f, "INNER ANY"),
            JoinType::Left => write!(f, "LEFT OUTER"),
            JoinType::LeftAny => write!(f, "LEFT ANY"),
            JoinType::Right => write!(f, "RIGHT OUTER"),
            JoinType::RightAny => write!(f, "RIGHT ANY"),
            JoinType::Full => write!(f, "FULL OUTER"),
            JoinType::LeftSemi => write!(f, "LEFT SEMI"),
            JoinType::LeftAnti => write!(f, "LEFT ANTI"),
            JoinType::RightSemi => write!(f, "RIGHT SEMI"),
            JoinType::RightAnti => write!(f, "RIGHT ANTI"),
            JoinType::Cross => write!(f, "CROSS"),
            JoinType::LeftMark => write!(f, "LEFT MARK"),
            JoinType::RightMark => write!(f, "RIGHT MARK"),
            JoinType::LeftSingle => write!(f, "LEFT SINGLE"),
            JoinType::RightSingle => write!(f, "RIGHT SINGLE"),
            JoinType::Asof => write!(f, "ASOF"),
            JoinType::LeftAsof => write!(f, "LEFT ASOF"),
            JoinType::RightAsof => write!(f, "RIGHT ASOF"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HashJoinBuildCacheInfo {
    pub cache_idx: usize,
    pub columns: Vec<Symbol>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericJoinEquiCondition<ScalarExpr> {
    pub left: ScalarExpr,
    pub right: ScalarExpr,
    pub is_null_equal: bool,
}

impl<ScalarExpr> GenericJoinEquiCondition<ScalarExpr> {
    pub fn new(left: ScalarExpr, right: ScalarExpr, is_null_equal: bool) -> Self {
        Self {
            left,
            right,
            is_null_equal,
        }
    }

    pub fn new_conditions(
        left: Vec<ScalarExpr>,
        right: Vec<ScalarExpr>,
        is_null_equal: Vec<usize>,
    ) -> Vec<Self> {
        left.into_iter()
            .zip(right)
            .enumerate()
            .map(|(index, (left, right))| Self {
                left,
                right,
                is_null_equal: is_null_equal.contains(&index),
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericJoin<ScalarExpr> {
    pub equi_conditions: Vec<GenericJoinEquiCondition<ScalarExpr>>,
    pub non_equi_conditions: Vec<ScalarExpr>,
    pub join_type: JoinType,
    pub marker_index: Option<Symbol>,
    pub from_correlated_subquery: bool,
    pub need_hold_hash_table: bool,
    pub is_lateral: bool,
    pub single_to_inner: Option<JoinType>,
    pub build_side_cache_info: Option<HashJoinBuildCacheInfo>,
}

impl<ScalarExpr> Default for GenericJoin<ScalarExpr> {
    fn default() -> Self {
        Self {
            equi_conditions: Vec::new(),
            non_equi_conditions: Vec::new(),
            join_type: JoinType::Cross,
            marker_index: None,
            from_correlated_subquery: false,
            need_hold_hash_table: false,
            is_lateral: false,
            single_to_inner: None,
            build_side_cache_info: None,
        }
    }
}

impl<ScalarExpr> GenericJoin<ScalarExpr>
where
    ScalarExpr: PlanScalarExpr,
{
    pub fn used_columns(&self) -> Result<ColumnSet> {
        let mut used_columns = ColumnSet::new();
        for condition in self.equi_conditions.iter() {
            used_columns = used_columns
                .union(&condition.left.used_columns())
                .cloned()
                .collect();
            used_columns = used_columns
                .union(&condition.right.used_columns())
                .cloned()
                .collect();
        }
        for condition in self.non_equi_conditions.iter() {
            used_columns = used_columns
                .union(&condition.used_columns())
                .cloned()
                .collect();
        }
        Ok(used_columns)
    }

    pub fn has_null_equi_condition(&self) -> bool {
        self.equi_conditions
            .iter()
            .any(|condition| condition.is_null_equal)
    }
}
