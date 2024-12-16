// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::gucs;
use crate::index::merge_policy::AllowedMergePolicy;
use crate::postgres::index::get_fields;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::schema::{SearchFieldConfig, SearchIndexSchema};
use anyhow::Result;
use pgrx::PgRelation;
use std::num::NonZeroUsize;
use tantivy::Index;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use tracing::trace;

pub enum WriterResources {
    CreateIndex,
    Statement,
    Vacuum,
}
pub type Parallelism = NonZeroUsize;
pub type MemoryBudget = usize;
pub type DoMerging = bool;
pub type IndexConfig = (Parallelism, MemoryBudget, DoMerging, AllowedMergePolicy);

pub enum BlockDirectoryType {
    Mvcc,
    BulkDelete,
}

impl WriterResources {
    pub fn resources(&self, index_options: &SearchIndexCreateOptions) -> IndexConfig {
        let target_segment_count = index_options.target_segment_count();
        let merge_on_insert = index_options.merge_on_insert();

        match self {
            WriterResources::CreateIndex => {
                (
                    gucs::create_index_parallelism(),
                    gucs::create_index_memory_budget(),
                    true, // we always want a merge on CREATE INDEX
                    AllowedMergePolicy::NPlusOne(target_segment_count),
                )
            }
            WriterResources::Statement => {
                let policy = if merge_on_insert {
                    AllowedMergePolicy::NPlusOne(target_segment_count)
                } else {
                    AllowedMergePolicy::None
                };
                (
                    gucs::statement_parallelism(),
                    gucs::statement_memory_budget(),
                    merge_on_insert, // user/index decides if we merge for INSERT/UPDATE statements
                    policy,
                )
            }
            WriterResources::Vacuum => {
                (
                    gucs::statement_parallelism(),
                    gucs::statement_memory_budget(),
                    true, // we always want a merge on (auto)VACUUM
                    AllowedMergePolicy::NPlusOne(target_segment_count),
                )
            }
        }
    }
}

pub fn get_index_schema(index_relation: &PgRelation) -> Result<SearchIndexSchema> {
    if index_relation.rd_options.is_null() {
        panic!("must specify key field")
    }
    let (fields, key_field_index) = unsafe { get_fields(index_relation) };
    let schema = SearchIndexSchema::new(fields, key_field_index)?;
    Ok(schema)
}

pub fn setup_tokenizers(underlying_index: &mut Index, schema: &SearchIndexSchema) {
    let tokenizers = schema
        .fields
        .iter()
        .filter_map(|field| {
            let field_config = &field.config;
            let field_name: &str = field.name.as_ref();
            trace!(field_name, "attempting to create tokenizer");
            match field_config {
                SearchFieldConfig::Text { tokenizer, .. }
                | SearchFieldConfig::Json { tokenizer, .. } => Some(tokenizer),
                _ => None,
            }
        })
        .collect();

    underlying_index.set_tokenizers(create_tokenizer_manager(tokenizers));
    underlying_index.set_fast_field_tokenizers(create_normalizer_manager());
}
