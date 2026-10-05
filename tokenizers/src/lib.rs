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

pub mod chinese_convert;
pub mod cjk;
pub mod code;
pub mod edge_ngram;
pub mod icu;
pub mod lindera;
pub mod manager;
pub mod ngram;
pub mod token_length;
pub mod token_trim;
mod unicode_words;

use tantivy::tokenizer::{LowerCaser, RawTokenizer, TextAnalyzer, TokenizerManager};
use tracing::debug;

pub use manager::{SearchNormalizer, SearchTokenizer};

pub fn create_tokenizer_manager(search_tokenizers: Vec<SearchTokenizer>) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();
    register_tokenizers_into(&tokenizer_manager, search_tokenizers);
    tokenizer_manager
}

/// Like [`create_tokenizer_manager`], for opening an existing index: also keeps every tokenizer
/// name the index's stored schema holds resolving as it did when the index was built. See
/// [`register_index_tokenizers_into`].
pub fn create_index_tokenizer_manager(
    search_tokenizers: Vec<SearchTokenizer>,
    stored_fields: &[StoredFieldTokenizer],
) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();
    register_index_tokenizers_into(&tokenizer_manager, search_tokenizers, stored_fields);
    tokenizer_manager
}

/// Register `search_tokenizers` into an existing manager. `TokenizerManager` registration
/// mutates its shared registry, so readers already holding the manager see the entries too.
pub fn register_tokenizers_into(
    tokenizer_manager: &TokenizerManager,
    search_tokenizers: Vec<SearchTokenizer>,
) {
    register_current_names(tokenizer_manager, &search_tokenizers);
}

/// A text field's tokenizer paired with the tokenizer name persisted in the index's stored
/// schema, which can differ from [`SearchTokenizer::name`] for an index built by an older version.
#[derive(Clone, Debug)]
pub struct StoredFieldTokenizer {
    pub tokenizer: SearchTokenizer,
    pub stored_name: String,
}

/// Register `search_tokenizers` for an existing index whose stored schema is described by
/// `stored_fields`.
///
/// Before the `trim` filter was part of [`SearchTokenizer::name`], fields differing only in
/// `trim` shared one stored name, and the registry's plain keyed insert made the *last*
/// tokenizer registered under it win for all of them -- when the index was built, and for every
/// query and insert since. Those already-indexed terms only stay consistent with later queries
/// and inserts if that name keeps resolving to that same last-wins analyzer, so for any stored
/// name the current naming no longer produces, replay the old resolution: the last tokenizer, in
/// registration order, whose pre-fix name is that stored name. An index built after the fix
/// stores the current names, so none of this applies and every field keeps its own analyzer;
/// that is also what a REINDEX of a legacy index produces.
pub fn register_index_tokenizers_into(
    tokenizer_manager: &TokenizerManager,
    search_tokenizers: Vec<SearchTokenizer>,
    stored_fields: &[StoredFieldTokenizer],
) {
    register_current_names(tokenizer_manager, &search_tokenizers);

    let mut restored = std::collections::HashSet::new();
    for stored in stored_fields {
        if stored.stored_name == stored.tokenizer.name() || !restored.insert(&stored.stored_name) {
            continue;
        }
        let winner = search_tokenizers
            .iter()
            .rev()
            .filter(|t| t.pre_trim_fix_name() == stored.stored_name)
            .find_map(|t| t.to_tantivy_tokenizer());
        if let Some(text_analyzer) = winner {
            debug!(
                tokenizer_name = &stored.stored_name,
                "restoring pre-trim-fix resolution of stored tokenizer name",
            );
            tokenizer_manager.register(&stored.stored_name, text_analyzer);
        }
    }
}

fn register_current_names(
    tokenizer_manager: &TokenizerManager,
    search_tokenizers: &[SearchTokenizer],
) {
    for search_tokenizer in search_tokenizers {
        if let Some(text_analyzer) = search_tokenizer.to_tantivy_tokenizer() {
            let name = search_tokenizer.name();
            debug!(tokenizer_name = &name, "registering tokenizer");
            tokenizer_manager.register(&name, text_analyzer);
        }
    }
}

pub fn create_normalizer_manager() -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::new();
    register_normalizers_into(&tokenizer_manager);
    tokenizer_manager
}

/// Register the fast-field normalizers into an existing manager; see
/// [`register_tokenizers_into`] for why registration rather than replacement.
pub fn register_normalizers_into(tokenizer_manager: &TokenizerManager) {
    let raw_tokenizer = TextAnalyzer::builder(RawTokenizer::default()).build();
    let lower_case_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(LowerCaser)
        .build();
    tokenizer_manager.register("raw", raw_tokenizer);
    tokenizer_manager.register("lowercase", lower_case_tokenizer);
}

#[cfg(test)]
mod legacy_trim_upgrade_tests {
    use super::*;
    use crate::manager::SearchTokenizerFilters;
    use rstest::rstest;
    use tantivy::collector::Count;
    use tantivy::query::TermQuery;
    use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
    use tantivy::tokenizer::TokenStream;
    use tantivy::{Index, Term, doc};

    // `LiteralNormalized` wraps `RawTokenizer`: the whole input is one token, so `trim` has an
    // observable effect. "  Hello  " lowercases to "  hello  " untrimmed, or "hello" trimmed.
    const TEXT: &str = "  Hello  ";
    const UNTRIMMED: &str = "  hello  ";
    const TRIMMED: &str = "hello";

    fn literal(trim: Option<bool>) -> SearchTokenizer {
        SearchTokenizer::LiteralNormalized(SearchTokenizerFilters {
            trim,
            ..SearchTokenizerFilters::default()
        })
    }

    fn schema_for(stored_names: &[&str]) -> Schema {
        let mut builder = Schema::builder();
        for (i, name) in stored_names.iter().enumerate() {
            let indexing = TextFieldIndexing::default()
                .set_tokenizer(name)
                .set_index_option(IndexRecordOption::Basic);
            builder.add_text_field(
                &format!("f{i}"),
                TextOptions::default().set_indexing_options(indexing),
            );
        }
        builder.build()
    }

    /// How the registry resolved names before the trim fix: registered under the name that
    /// ignores `trim`, so the last tokenizer registered under a shared name won for every field.
    fn pre_fix_manager(tokenizers: &[SearchTokenizer]) -> TokenizerManager {
        let manager = TokenizerManager::default();
        for tokenizer in tokenizers {
            manager.register(
                &tokenizer.pre_trim_fix_name(),
                tokenizer.to_tantivy_tokenizer().unwrap(),
            );
        }
        manager
    }

    fn tokens(manager: &TokenizerManager, name: &str) -> Vec<String> {
        let mut analyzer = manager.get(name).unwrap();
        let mut stream = analyzer.token_stream(TEXT);
        let mut out = Vec::new();
        while stream.advance() {
            out.push(stream.token().text.clone());
        }
        out
    }

    fn hits(index: &Index, field: usize, term: &str) -> usize {
        let reader = index.reader().unwrap();
        reader.reload().unwrap();
        let field = index.schema().get_field(&format!("f{field}")).unwrap();
        let query = TermQuery::new(Term::from_field_text(field, term), IndexRecordOption::Basic);
        reader.searcher().search(&query, &Count).unwrap()
    }

    fn insert(index: &Index, fields: usize) {
        let schema = index.schema();
        let mut writer = index.writer(15_000_000).unwrap();
        let mut document = doc!();
        for i in 0..fields {
            document.add_text(schema.get_field(&format!("f{i}")).unwrap(), TEXT);
        }
        writer.add_document(document).unwrap();
        writer.commit().unwrap();
    }

    /// An index built before the fix whose two fields share one stored name must keep resolving
    /// that name to the analyzer the old registry picked -- the last tokenizer registered --
    /// for queries and inserts alike, so new terms agree with already-indexed ones. After a
    /// REINDEX the stored names are the current ones and each field has its own analyzer.
    #[rstest]
    #[case::plain_then_trim_true(literal(None), literal(Some(true)), TRIMMED)]
    #[case::trim_false_then_trim_true(literal(Some(false)), literal(Some(true)), TRIMMED)]
    #[case::trim_true_then_plain(literal(Some(true)), literal(None), UNTRIMMED)]
    #[case::trim_true_then_trim_false(literal(Some(true)), literal(Some(false)), UNTRIMMED)]
    fn legacy_index_keeps_original_analyzer_until_reindex(
        #[case] first: SearchTokenizer,
        #[case] second: SearchTokenizer,
        #[case] original_term: &str,
    ) {
        let fields = vec![first, second];
        let stored_name = fields[0].pre_trim_fix_name();
        assert_eq!(stored_name, fields[1].pre_trim_fix_name());
        let other_term = if original_term == TRIMMED {
            UNTRIMMED
        } else {
            TRIMMED
        };

        // Built by the old code: both fields store the shared pre-fix name.
        let mut legacy = Index::create_in_ram(schema_for(&[&stored_name, &stored_name]));
        legacy.set_tokenizers(pre_fix_manager(&fields));
        insert(&legacy, 2);
        for field in 0..2 {
            assert_eq!(hits(&legacy, field, original_term), 1);
            assert_eq!(hits(&legacy, field, other_term), 0);
        }

        // Opened by the new code with the stored schema.
        let stored_fields: Vec<_> = fields
            .iter()
            .map(|tokenizer| StoredFieldTokenizer {
                tokenizer: tokenizer.clone(),
                stored_name: stored_name.clone(),
            })
            .collect();
        let upgraded = create_index_tokenizer_manager(fields.clone(), &stored_fields);
        assert_eq!(
            tokens(&upgraded, &stored_name),
            vec![original_term.to_string()],
            "queries must tokenize with the analyzer the already-indexed terms came from"
        );
        legacy.set_tokenizers(upgraded);
        insert(&legacy, 2);
        for field in 0..2 {
            assert_eq!(
                hits(&legacy, field, original_term),
                2,
                "an insert before REINDEX must produce the same terms as the existing rows"
            );
            assert_eq!(hits(&legacy, field, other_term), 0);
        }

        // REINDEX: the schema now stores the current names, so the fields are independent.
        let current_names: Vec<_> = fields.iter().map(|t| t.name()).collect();
        let mut reindexed =
            Index::create_in_ram(schema_for(&[&current_names[0], &current_names[1]]));
        let stored_fields: Vec<_> = fields
            .iter()
            .map(|tokenizer| StoredFieldTokenizer {
                tokenizer: tokenizer.clone(),
                stored_name: tokenizer.name(),
            })
            .collect();
        reindexed.set_tokenizers(create_index_tokenizer_manager(
            fields.clone(),
            &stored_fields,
        ));
        insert(&reindexed, 2);
        for (field, tokenizer) in fields.iter().enumerate() {
            let own_term = if tokenizer.filters_trim() == Some(true) {
                TRIMMED
            } else {
                UNTRIMMED
            };
            let foreign_term = if own_term == TRIMMED {
                UNTRIMMED
            } else {
                TRIMMED
            };
            assert_eq!(hits(&reindexed, field, own_term), 1);
            assert_eq!(hits(&reindexed, field, foreign_term), 0);
        }
    }

    /// A tokenizer manager built without a stored schema (SQL casts, query-time tokenizers)
    /// registers only current names; nothing is aliased.
    #[rstest]
    fn tokenizers_without_a_stored_schema_get_no_legacy_names() {
        let trim = literal(Some(true));
        let manager = create_tokenizer_manager(vec![trim.clone()]);
        assert!(manager.get(&trim.name()).is_some());
        assert!(manager.get(&trim.pre_trim_fix_name()).is_none());
    }

    /// A new-style index stores current names; a plain field and a trim field keep their own
    /// analyzers even though the trim field's pre-fix name equals the plain field's name.
    #[rstest]
    fn new_style_index_keeps_fields_independent() {
        let plain = literal(None);
        let trim = literal(Some(true));
        let stored_fields = vec![
            StoredFieldTokenizer {
                tokenizer: plain.clone(),
                stored_name: plain.name(),
            },
            StoredFieldTokenizer {
                tokenizer: trim.clone(),
                stored_name: trim.name(),
            },
        ];
        let manager =
            create_index_tokenizer_manager(vec![plain.clone(), trim.clone()], &stored_fields);
        assert_eq!(tokens(&manager, &plain.name()), vec![UNTRIMMED.to_string()]);
        assert_eq!(tokens(&manager, &trim.name()), vec![TRIMMED.to_string()]);
    }
}
