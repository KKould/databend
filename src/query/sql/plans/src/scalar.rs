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
use databend_common_expression::RemoteExpr;
use databend_common_expression::Scalar;
use databend_common_expression::types::DataType;
use educe::Educe;

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericLambdaFunc<ScalarExpr> {
    #[educe(PartialEq(ignore), Hash(ignore))]
    pub span: Span,
    pub func_name: String,
    pub args: Vec<ScalarExpr>,
    pub lambda_expr: Box<RemoteExpr>,
    pub lambda_display: String,
    pub return_type: Box<DataType>,
}

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericFunctionCall<ScalarExpr> {
    #[educe(Hash(ignore), PartialEq(ignore))]
    pub span: Span,
    pub func_name: String,
    pub params: Vec<Scalar>,
    pub arguments: Vec<ScalarExpr>,
}

#[derive(Clone, Debug, Educe)]
#[educe(PartialEq, Eq, Hash)]
pub struct GenericCastExpr<ScalarExpr> {
    #[educe(Hash(ignore), PartialEq(ignore))]
    pub span: Span,
    pub is_try: bool,
    pub argument: Box<ScalarExpr>,
    pub target_type: Box<DataType>,
}
