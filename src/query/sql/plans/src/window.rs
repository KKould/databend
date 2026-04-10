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

use databend_common_ast::Span;
use databend_common_exception::ErrorCode;
use databend_common_exception::Result;
use databend_common_expression::Scalar;
use databend_common_expression::types::DataType;
use databend_common_expression::types::NumberDataType;
use educe::Educe;
use enum_as_inner::EnumAsInner;
use serde::Deserialize;
use serde::Serialize;

use crate::ColumnSet;
use crate::GenericScalarItem;
use crate::GenericWindowOrderByInfo;
use crate::PlanScalarExpr;

pub trait PlanWindowAggregateFunction<ScalarExpr> {
    fn func_name(&self) -> &str;
    fn exprs(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_>;
    fn return_type(&self) -> DataType;
}

pub trait PlanWindowLagLeadFunction<ScalarExpr> {
    fn is_lag(&self) -> bool;
    fn arg(&self) -> &ScalarExpr;
    fn default(&self) -> Option<&ScalarExpr>;
    fn return_type(&self) -> DataType;
}

pub trait PlanWindowNthValueFunction<ScalarExpr> {
    fn arg(&self) -> &ScalarExpr;
    fn return_type(&self) -> DataType;
}

pub trait PlanWindowNtileFunction {
    fn return_type(&self) -> DataType;
}

pub trait PlanWindowFunction<ScalarExpr> {
    fn func_name(&self) -> String;
    fn used_columns(&self) -> ColumnSet;
    fn return_type(&self) -> DataType;
    fn scalar_expr_iter(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_>;
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericLagLeadFunction<ScalarExpr> {
    /// Is `lag` or `lead`.
    pub is_lag: bool,
    pub arg: Box<ScalarExpr>,
    pub offset: u64,
    pub default: Option<Box<ScalarExpr>>,
    pub return_type: Box<DataType>,
}

impl<ScalarExpr> PlanWindowLagLeadFunction<ScalarExpr> for GenericLagLeadFunction<ScalarExpr> {
    fn is_lag(&self) -> bool {
        self.is_lag
    }

    fn arg(&self) -> &ScalarExpr {
        self.arg.as_ref()
    }

    fn default(&self) -> Option<&ScalarExpr> {
        self.default.as_deref()
    }

    fn return_type(&self) -> DataType {
        (*self.return_type).clone()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericNthValueFunction<ScalarExpr> {
    /// The nth row of the window frame (counting from 1).
    ///
    /// - Some(1): `first_value`
    /// - Some(n): `nth_value`
    /// - None: `last_value`
    pub n: Option<u64>,
    pub arg: Box<ScalarExpr>,
    pub return_type: Box<DataType>,
    pub ignore_null: bool,
}

impl<ScalarExpr> PlanWindowNthValueFunction<ScalarExpr> for GenericNthValueFunction<ScalarExpr> {
    fn arg(&self) -> &ScalarExpr {
        self.arg.as_ref()
    }

    fn return_type(&self) -> DataType {
        (*self.return_type).clone()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericNtileFunction {
    pub n: u64,
    pub return_type: Box<DataType>,
}

impl PlanWindowNtileFunction for GenericNtileFunction {
    fn return_type(&self) -> DataType {
        (*self.return_type).clone()
    }
}

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericWindowFunc<WindowFuncType, ScalarExpr> {
    #[educe(PartialEq(ignore), Hash(ignore))]
    pub span: Span,
    pub display_name: String,
    pub partition_by: Vec<ScalarExpr>,
    pub func: WindowFuncType,
    pub order_by: Vec<GenericWindowOrderBy<ScalarExpr>>,
    pub frame: WindowFuncFrame,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GenericWindowOrderBy<ScalarExpr> {
    pub expr: ScalarExpr,
    // Optional `ASC` or `DESC`
    pub asc: Option<bool>,
    // Optional `NULLS FIRST` or `NULLS LAST`
    pub nulls_first: Option<bool>,
}

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericWindow<WindowFuncType, ScalarExpr> {
    #[educe(PartialEq(ignore), Hash(ignore))]
    pub span: Span,

    // aggregate scalar expressions, such as: sum(col1), count(*);
    // or general window functions, such as: row_number(), rank();
    pub index: crate::Symbol,
    pub function: WindowFuncType,
    pub arguments: Vec<GenericScalarItem<ScalarExpr>>,

    // partition by scalar expressions
    pub partition_by: Vec<GenericScalarItem<ScalarExpr>>,
    // order by
    pub order_by: Vec<GenericWindowOrderByInfo<ScalarExpr>>,
    // window frames
    pub frame: WindowFuncFrame,
    // limit for potentially possible push-down
    pub limit: Option<usize>,
}

impl<WindowFuncType, ScalarExpr> GenericWindow<WindowFuncType, ScalarExpr>
where
    WindowFuncType: PlanWindowFunction<ScalarExpr>,
    ScalarExpr: PlanScalarExpr,
{
    pub fn used_columns(&self) -> Result<ColumnSet> {
        let mut used_columns = ColumnSet::new();

        used_columns.insert(self.index);
        used_columns.extend(self.function.used_columns());
        used_columns.extend(self.arguments_columns()?);
        used_columns.extend(self.partition_by_columns()?);
        used_columns.extend(self.order_by_columns()?);

        Ok(used_columns)
    }

    pub fn arguments_columns(&self) -> Result<ColumnSet> {
        let mut col_set = ColumnSet::new();
        for arg in self.arguments.iter() {
            col_set.insert(arg.index);
            col_set.extend(arg.scalar.used_columns())
        }
        Ok(col_set)
    }

    pub fn partition_by_columns(&self) -> Result<ColumnSet> {
        let mut col_set = ColumnSet::new();
        for part in self.partition_by.iter() {
            col_set.insert(part.index);
            col_set.extend(part.scalar.used_columns())
        }
        Ok(col_set)
    }

    pub fn order_by_columns(&self) -> Result<ColumnSet> {
        let mut col_set = ColumnSet::new();
        for sort in self.order_by.iter() {
            col_set.insert(sort.order_by_item.index);
            col_set.extend(sort.order_by_item.scalar.used_columns())
        }
        Ok(col_set)
    }

    pub fn scalar_expr_iter(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_> {
        let iter = self.order_by.iter().map(|o| &o.order_by_item.scalar);
        let iter = iter.chain(self.partition_by.iter().map(|expr| &expr.scalar));
        let iter = iter.chain(self.arguments.iter().map(|expr| &expr.scalar));
        Box::new(iter.chain(self.function.scalar_expr_iter()))
    }
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct WindowFuncFrame {
    pub units: WindowFuncFrameUnits,
    pub start_bound: WindowFuncFrameBound,
    pub end_bound: WindowFuncFrameBound,
}

impl Display for WindowFuncFrame {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:?}: {:?} ~ {:?}",
            self.units, self.start_bound, self.end_bound
        )
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumAsInner)]
pub enum WindowFuncFrameUnits {
    #[default]
    Rows,
    Range,
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum WindowFuncFrameBound {
    /// `CURRENT ROW`
    #[default]
    CurrentRow,
    /// `<N> PRECEDING` or `UNBOUNDED PRECEDING`
    Preceding(Option<Scalar>),
    /// `<N> FOLLOWING` or `UNBOUNDED FOLLOWING`.
    Following(Option<Scalar>),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum GenericWindowFuncType<AggregateFunction, LagLeadFunction, NthValueFunction, NtileFunction>
{
    Aggregate(AggregateFunction),
    RowNumber,
    Rank,
    DenseRank,
    PercentRank,
    LagLead(LagLeadFunction),
    NthValue(NthValueFunction),
    Ntile(NtileFunction),
    CumeDist,
}

impl<AggregateFunction, LagLeadFunction, NthValueFunction, NtileFunction>
    GenericWindowFuncType<AggregateFunction, LagLeadFunction, NthValueFunction, NtileFunction>
{
    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "row_number" => Ok(Self::RowNumber),
            "rank" => Ok(Self::Rank),
            "dense_rank" => Ok(Self::DenseRank),
            "percent_rank" => Ok(Self::PercentRank),
            "cume_dist" => Ok(Self::CumeDist),
            _ => Err(ErrorCode::UnknownFunction(format!(
                "Unknown window function: {}",
                name
            ))),
        }
    }
}

impl<AggregateFunction, LagLeadFunction, NthValueFunction, NtileFunction, ScalarExpr>
    PlanWindowFunction<ScalarExpr>
    for GenericWindowFuncType<AggregateFunction, LagLeadFunction, NthValueFunction, NtileFunction>
where
    AggregateFunction: PlanWindowAggregateFunction<ScalarExpr>,
    LagLeadFunction: PlanWindowLagLeadFunction<ScalarExpr>,
    NthValueFunction: PlanWindowNthValueFunction<ScalarExpr>,
    NtileFunction: PlanWindowNtileFunction,
    ScalarExpr: PlanScalarExpr,
{
    fn func_name(&self) -> String {
        match self {
            Self::Aggregate(agg) => agg.func_name().to_string(),
            Self::RowNumber => "row_number".to_string(),
            Self::Rank => "rank".to_string(),
            Self::DenseRank => "dense_rank".to_string(),
            Self::PercentRank => "percent_rank".to_string(),
            Self::LagLead(lag_lead) if lag_lead.is_lag() => "lag".to_string(),
            Self::LagLead(_) => "lead".to_string(),
            Self::NthValue(_) => "nth_value".to_string(),
            Self::Ntile(_) => "ntile".to_string(),
            Self::CumeDist => "cume_dist".to_string(),
        }
    }

    fn used_columns(&self) -> ColumnSet {
        match self {
            Self::Aggregate(agg) => agg.exprs().flat_map(|expr| expr.used_columns()).collect(),
            Self::LagLead(func) => {
                let mut used_columns = func.arg().used_columns();
                if let Some(default) = func.default() {
                    used_columns.extend(default.used_columns());
                }
                used_columns
            }
            Self::NthValue(func) => func.arg().used_columns(),
            _ => ColumnSet::new(),
        }
    }

    fn return_type(&self) -> DataType {
        match self {
            Self::Aggregate(agg) => agg.return_type(),
            Self::RowNumber | Self::Rank | Self::DenseRank => {
                DataType::Number(NumberDataType::UInt64)
            }
            Self::PercentRank | Self::CumeDist => DataType::Number(NumberDataType::Float64),
            Self::LagLead(lag_lead) => lag_lead.return_type(),
            Self::NthValue(nth_value) => nth_value.return_type(),
            Self::Ntile(buckets) => buckets.return_type(),
        }
    }

    fn scalar_expr_iter(&self) -> Box<dyn Iterator<Item = &ScalarExpr> + '_> {
        match self {
            Self::Aggregate(agg) => agg.exprs(),
            Self::LagLead(func) => Box::new(std::iter::once(func.arg()).chain(func.default())),
            Self::NthValue(func) => Box::new(std::iter::once(func.arg())),
            _ => Box::new(std::iter::empty()),
        }
    }
}
