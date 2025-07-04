// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::postgres::rel::PgSearchRelation;
use anyhow::Result;
use tantivy::Index;
use tokenizers::{create_normalizer_manager, create_tokenizer_manager, SearchTokenizer};

pub fn setup_tokenizers(index_relation: &PgSearchRelation, index: &mut Index) -> Result<()> {
    let schema = index_relation.schema()?;
    let categorized_fields = schema.categorized_fields();

    let mut tokenizers: Vec<SearchTokenizer> = Vec::new();
    for (search_field, _) in categorized_fields.iter() {
        if search_field.is_ctid() {
            continue;
        }

        let config = search_field.field_config();
        if let Some(tokenizer) = config.tokenizer() {
            tokenizers.push(tokenizer.clone());
        }
    }

    index.set_tokenizers(create_tokenizer_manager(tokenizers));
    index.set_fast_field_tokenizers(create_normalizer_manager());
    Ok(())
}
