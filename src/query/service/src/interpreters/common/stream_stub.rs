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

use databend_common_exception::Result;
use databend_common_meta_app::schema::TableInfo;
use databend_common_meta_app::schema::UpdateStreamMetaReq;
use databend_common_meta_app::schema::UpdateTableMetaReq;

use crate::sessions::QueryContext;

pub struct StreamTableUpdates {
    pub update_table_metas: Vec<(UpdateTableMetaReq, TableInfo)>,
}

pub async fn dml_build_update_stream_req(
    _ctx: Arc<QueryContext>,
) -> Result<Vec<UpdateStreamMetaReq>> {
    Ok(vec![])
}

pub async fn query_build_update_stream_req(
    _ctx: &Arc<QueryContext>,
) -> Result<Option<StreamTableUpdates>> {
    Ok(None)
}
