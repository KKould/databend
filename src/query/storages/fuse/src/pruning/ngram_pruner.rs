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

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use log::{info, warn};
use opendal::Operator;
use tantivy::tokenizer::TokenizerManager;
use databend_common_expression::{ColumnId, Expr, FunctionContext, Scalar, TableDataType, TableField, TableSchema, TableSchemaRef, Value};
use databend_common_expression::types::Buffer;
use databend_common_meta_app::schema::TableIndex;
use databend_common_sql::BloomIndexColumns;
use databend_common_exception::Result;
use databend_storages_common_index::{BloomIndex, FilterEvalResult, NgramIndex};
use databend_storages_common_index::filters::BlockFilter;
use databend_storages_common_table_meta::meta::{BlockMeta, Location, StatisticsOfColumns};
use crate::io::{create_tokenizer_manager, BlockWriter, BloomBlockFilterReader, BloomIndexBuilder, NgramIndexBuilder};
use crate::pruning::{BloomPruner, BloomPrunerCreator};

pub struct NgramPrunerCreator {
    func_ctx: FunctionContext,

    index_fields: Vec<TableField>,

    filter_expression: Expr<String>,

    scalar_map: HashMap<Scalar, Buffer<u64>>,

    dal: Operator,

    data_schema: TableSchemaRef,

    ngram_index_builder: Option<NgramIndexBuilder>,

    n: u32,
}

impl NgramPrunerCreator {
    pub fn create(
        func_ctx: FunctionContext,
        schema: &TableSchemaRef,
        dal: Operator,
        filter_expr: Option<&Expr<String>>,
        ngram_index_cols: BloomIndexColumns,
        ngram_index_builder: Option<NgramIndexBuilder>,
        n: u32,
    ) -> Result<Option<Arc<dyn BloomPruner + Send + Sync>>> {
        let Some(expr) = filter_expr else {
            return Ok(None);
        };
        let ngram_columns_map = ngram_index_cols.bloom_index_fields(schema.clone(), |ty| matches!(ty.remove_nullable(), TableDataType::String))?;
        let ngram_column_fields = ngram_columns_map.values().cloned().collect::<Vec<_>>();
        let (index_fields, scalars) =
            NgramIndex::filter_index_field(expr.clone(), &ngram_column_fields)?;

        if index_fields.is_empty() {
            return Ok(None);
        }

        let mut scalar_map = HashMap::<Scalar, Buffer<u64>>::new();
        for (scalar, ty) in scalars.into_iter() {
            if let Entry::Vacant(e) = scalar_map.entry(scalar) {
                if let Some(digests) = NgramIndex::calculate_nullable_column_digest(&func_ctx, Value::Scalar(e.key().clone()), &ty, n)?.next() {
                    e.insert(digests);
                }
            }
        }

        Ok(Some(Arc::new(Self {
            func_ctx,
            index_fields,
            filter_expression: expr.clone(),
            scalar_map,
            dal,
            data_schema: schema.clone(),
            ngram_index_builder,
            n,
        })))
    }

    #[async_backtrace::framed]
    pub async fn apply(
        &self,
        index_location: &Location,
        index_length: u64,
        column_stats: &StatisticsOfColumns,
        column_ids_of_indexed_block: Vec<ColumnId>,
        block_meta: &BlockMeta,
    ) -> Result<bool> {
        let index_columns = self.index_fields.iter().fold(
            Vec::with_capacity(self.index_fields.len()),
            |mut acc, field| {
                if column_ids_of_indexed_block.contains(&field.column_id()) {
                    acc.push(NgramIndex::build_filter_column_name(field));
                }
                acc
            }
        );

        let maybe_filter = index_location
            .read_block_filter(self.dal.clone(), &index_columns, index_length)
            .await;

        let maybe_filter = match (&maybe_filter, &self.ngram_index_builder) {
            (Err(_e), Some(ngram_index_builder)) => {
                match self
                    .try_rebuild_missing_ngram_index(&block_meta, ngram_index_builder, &index_columns).await
                {
                    Ok(Some(block_filter)) => Ok(block_filter),
                    Ok(None) => maybe_filter,
                    Err(e) => {
                        info!(
                            "failed to re-build missing index at location {:?}, {}",
                            index_location, e
                        );
                        maybe_filter
                    }
                }
            }
            _ => maybe_filter,
        };

        match maybe_filter {
            Ok(filter) => Ok(NgramIndex::from_filter_block(
                self.func_ctx.clone(),
                filter.filter_schema,
                filter.filters,
                self.n,
            )?.apply(
                self.filter_expression.clone(),
                &self.scalar_map,
                column_stats,
                self.data_schema.clone(),
            )? != FilterEvalResult::MustFalse),
            Err(e) => Err(e),
        }
    }

    async fn try_rebuild_missing_ngram_index(
        &self,
        block_meta: &BlockMeta,
        ngram_index_builder: &NgramIndexBuilder,
        index_columns: &[String],
    ) -> Result<Option<BlockFilter>> {
        let Some(ngram_index_location) = &block_meta.ngram_filter_index_location else {
            info!("no ngram index found in block meta, ignore");
            return Ok(None);
        };

        if self.dal.exists(ngram_index_location.0.as_ref()).await? {
            info!("ngram index exists, ignore");
            return Ok(None);
        }

        let ngram_index_state = ngram_index_builder
            .ngram_index_state_from_block_meta(block_meta)
            .await?;

        if let Some((ngram_state, ngram_index)) = ngram_index_state {
            let column_needed: HashSet<&String> = HashSet::from_iter(index_columns.iter());
            let indexed_fields = &ngram_index.filter_schema.fields;
            let mut new_filter_schema_fields = Vec::new();
            let mut filters = Vec::new();

            for (idx, field) in indexed_fields.iter().enumerate() {
                for column_name in &column_needed {
                    if &field.name == *column_name {
                        if let Some(filter) = ngram_index.filters.get(idx) {
                            new_filter_schema_fields.push(field.clone());
                            filters.push(filter.clone())
                        }
                    }
                }
            }

            BlockWriter::write_down_ngram_index_state(
                &ngram_index_builder.table_dal,
                Some(ngram_state),
            ).await?;

            info!("re-created missing index {:?}", ngram_index_location);

            Ok(Some(BlockFilter {
                filter_schema: Arc::new(TableSchema::new(new_filter_schema_fields)),
                filters,
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl BloomPruner for NgramPrunerCreator {
    #[async_backtrace::framed]
    async fn should_keep(
        &self,
        index_location: &Option<Location>,
        index_length: u64,
        column_stats: &StatisticsOfColumns,
        column_ids: Vec<ColumnId>,
        block_meta: &BlockMeta
    ) -> bool {
        let Some(loc) = index_location else {
            return true
        };
        self.apply(loc, index_length, column_stats, column_ids, block_meta)
            .await.unwrap_or_else(|e| {
            warn!("failed to apply bloom pruner, returning true. {}", e);
            true
        })
    }
}
