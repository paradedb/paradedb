// Copyright (c) 2023-2025 Retake, Inc.
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
use crate::postgres::index::get_fields;
use crate::schema::{SearchFieldConfig, SearchIndexSchema};
use anyhow::Result;
use pgrx::PgRelation;
use std::num::NonZeroUsize;
use tantivy::Index;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager};

pub enum WriterResources {
    CreateIndex,
    Statement,
    PostStatementMerge,
    Vacuum,
}
pub type Parallelism = NonZeroUsize;
pub type MemoryBudget = usize;
pub type IndexConfig = (Parallelism, MemoryBudget);

impl WriterResources {
    pub fn resources(&self) -> IndexConfig {
        match self {
            WriterResources::CreateIndex => (
                gucs::create_index_parallelism(),
                gucs::create_index_memory_budget(),
            ),
            WriterResources::Statement => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
            ),
            WriterResources::PostStatementMerge => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
            ),
            WriterResources::Vacuum => (
                gucs::statement_parallelism(),
                gucs::statement_memory_budget(),
            ),
        }
    }
}

pub fn get_index_schema(index_relation: &PgRelation) -> Result<SearchIndexSchema> {
    let (fields, key_field_index) = unsafe { get_fields(index_relation) };
    let schema = SearchIndexSchema::new(fields, key_field_index)?;
    Ok(schema)
}

pub fn setup_tokenizers(underlying_index: &mut Index, index_relation: &PgRelation) {
    let (fields, _) = unsafe { get_fields(index_relation) };
    let tokenizers = fields
        .iter()
        .filter_map(|(_field_name, field_config, _)| match field_config {
            SearchFieldConfig::Text { tokenizer, .. }
            | SearchFieldConfig::Json { tokenizer, .. } => Some(tokenizer),
            _ => None,
        })
        .collect();

    underlying_index.set_tokenizers(create_tokenizer_manager(tokenizers));
    underlying_index.set_fast_field_tokenizers(create_normalizer_manager());
}
