// Copyright (c) 2023-2026 ParadeDB, Inc.
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
use tokenizers::manager::SearchTokenizerFilters;
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

            // <= `0.20.5`, `unicode_words` was accidentally named `remove_emojis`, so we need to register the old name for backwards compatibility
            if let SearchTokenizer::UnicodeWords {
                remove_emojis,
                filters,
            } = tokenizer
            {
                tokenizers.push(SearchTokenizer::UnicodeWordsDeprecated {
                    remove_emojis: *remove_emojis,
                    filters: filters.clone(),
                });
            }
        }

        if let Some(search_tokenizer) = config.search_tokenizer() {
            tokenizers.push(search_tokenizer.clone());
        }
    }

    // In 0.19.0 we changed the default `remove_long` filter for the keyword tokenizer from `usize::MAX` to `None`
    // As such, the tokenizer name of `keyword` went from `keyword[remove_long=...]` to just `keyword[...]`
    // so this is necessary to maintain backwards compatibility with existing indexes
    #[allow(deprecated)]
    tokenizers.push(SearchTokenizer::KeywordDeprecated);
    #[allow(deprecated)]
    tokenizers.push(SearchTokenizer::Raw(
        SearchTokenizerFilters::keyword_deprecated().clone(),
    ));
    // In 0.20.0 we changed the default tokenizer from `simple` to `unicode_words`
    tokenizers.push(SearchTokenizer::Simple(SearchTokenizerFilters::default()));

    index.set_tokenizers(create_tokenizer_manager(tokenizers));
    index.set_fast_field_tokenizers(create_normalizer_manager());
    Ok(())
}
