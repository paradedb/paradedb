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
use tokenizers::{
    create_normalizer_manager, create_tokenizer_manager, register_normalizers_into,
    register_tokenizers_into, SearchTokenizer,
};

/// Install this index's tokenizers on a freshly opened `index` by replacing its managers.
/// Only correct before any reader is built from `index`: replacement swaps this instance's
/// manager and leaves already-built readers holding the old one.
///
/// Reader opens defer this setup until the query reports that it needs tokenization, avoiding
/// analyzer registration for queries that never use it. Index writers still call it eagerly
/// while constructing the index, before any reader exists.
pub fn setup_tokenizers(index_relation: &PgSearchRelation, index: &mut Index) -> Result<()> {
    let tokenizers = collect_search_tokenizers(index_relation)?;
    index.set_tokenizers(create_tokenizer_manager(tokenizers));
    index.set_fast_field_tokenizers(create_normalizer_manager());
    Ok(())
}

/// Install this index's tokenizers by registering into the managers `index` already shares
/// with live readers (e.g. a manifest's searcher). Registration mutates the shared registry,
/// so readers built before this call see the entries too. Unlike [`setup_tokenizers`], the
/// managers keep tantivy's default entries alongside ours; lookups are by name, so the extra
/// entries are inert.
///
/// Manifest capture deliberately builds its searcher without custom tokenizers. If the eventual
/// query needs them, reader construction calls this method lazily because replacing the managers
/// after that searcher exists would not update the managers it already holds.
pub fn register_tokenizers(index_relation: &PgSearchRelation, index: &Index) -> Result<()> {
    let tokenizers = collect_search_tokenizers(index_relation)?;
    register_tokenizers_into(index.tokenizers(), tokenizers);
    register_normalizers_into(index.fast_field_tokenizer());
    Ok(())
}

/// Every tokenizer this index's fields can reference at query time, including the
/// deprecated-name aliases older index versions were built with.
fn collect_search_tokenizers(index_relation: &PgSearchRelation) -> Result<Vec<SearchTokenizer>> {
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

            match tokenizer {
                // <= `0.20.5`, `unicode_words` was accidentally named `remove_emojis`, so we need to register the old name for backwards compatibility
                SearchTokenizer::UnicodeWords {
                    remove_emojis,
                    filters,
                } => {
                    tokenizers.push(SearchTokenizer::UnicodeWordsDeprecated {
                        remove_emojis: *remove_emojis,
                        filters: filters.clone(),
                    });
                }

                // <= `0.22.3`, Lindera tokenizers did not support user-specified keep_whitespace. They defaulted to true.
                // Going forward, they default to false (to match Lindera behavior), but we need to register the old ones
                // (that still default to false) for backwards compatibility
                SearchTokenizer::ChineseLindera {
                    filters,
                    keep_whitespace: _,
                } => {
                    tokenizers.push(SearchTokenizer::ChineseLinderaDeprecated(filters.clone()));
                }
                SearchTokenizer::JapaneseLindera {
                    filters,
                    keep_whitespace: _,
                } => {
                    tokenizers.push(SearchTokenizer::JapaneseLinderaDeprecated(filters.clone()));
                }
                SearchTokenizer::KoreanLindera {
                    filters,
                    keep_whitespace: _,
                } => {
                    tokenizers.push(SearchTokenizer::KoreanLinderaDeprecated(filters.clone()));
                }
                SearchTokenizer::Lindera {
                    language, filters, ..
                } => {
                    tokenizers.push(SearchTokenizer::LinderaDeprecated(
                        language.clone(),
                        filters.clone(),
                    ));
                }
                _ => {}
            }
        }

        if let Some(search_tokenizer) = config.search_tokenizer() {
            tokenizers.push(search_tokenizer.clone());
        }
    }

    if let Some(index_search_tokenizer) = index_relation.options().search_tokenizer() {
        tokenizers.push(index_search_tokenizer);
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

    Ok(tokenizers)
}
