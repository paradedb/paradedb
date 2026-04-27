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

/// Force-load every dictionary-backed tokenizer so postmaster-forked workers
/// inherit them via COW.
pub fn prewarm_dictionary_tokenizers() {
    lindera::prewarm();

    // Constructing a JiebaTokenizer is a no-op; only token_stream() forces
    // the embedded JIEBA lazy_static.
    use tantivy::tokenizer::{TokenStream, Tokenizer};
    let mut t = tantivy_jieba::JiebaTokenizer::with_ordinal_position_mode(true);
    let mut stream = t.token_stream("warm");
    while stream.advance() {}

    chinese_convert::prewarm();
}

pub fn create_tokenizer_manager(search_tokenizers: Vec<SearchTokenizer>) -> TokenizerManager {
    let tokenizer_manager = TokenizerManager::default();

    for search_tokenizer in search_tokenizers {
        let tokenizer_option = search_tokenizer.to_tantivy_tokenizer();

        if let Some(text_analyzer) = tokenizer_option {
            debug!(
                tokenizer_name = &search_tokenizer.name(),
                "registering tokenizer",
            );
            tokenizer_manager.register(&search_tokenizer.name(), text_analyzer);
        }
    }

    tokenizer_manager
}

pub fn create_normalizer_manager() -> TokenizerManager {
    let raw_tokenizer = TextAnalyzer::builder(RawTokenizer::default()).build();
    let lower_case_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
        .filter(LowerCaser)
        .build();
    let tokenizer_manager = TokenizerManager::new();
    tokenizer_manager.register("raw", raw_tokenizer);
    tokenizer_manager.register("lowercase", lower_case_tokenizer);
    tokenizer_manager
}

#[cfg(test)]
mod prewarm_4840_tests {
    /// Catches the regression class where someone "simplifies"
    /// `prewarm_dictionary_tokenizers` and silently drops a dictionary
    /// family — for example by deleting the `token_stream("warm")` call
    /// (constructing a `JiebaTokenizer` alone is a no-op and was the
    /// original jieba bug review found).
    #[test]
    fn prewarm_body_covers_every_dict_tokenizer_family() {
        let src = include_str!("lib.rs");
        // Slice exactly the function body, not the rest of the file. If we
        // scanned a fixed window we'd cover this test's own assertion
        // string literals and the test would pass even when the prewarm
        // calls had been deleted.
        let header_pos = src
            .find("pub fn prewarm_dictionary_tokenizers()")
            .expect("prewarm_dictionary_tokenizers must exist");
        let body_open = header_pos
            + src[header_pos..]
                .find('{')
                .expect("function body must open with {");
        let body_end = {
            let mut depth = 0i32;
            let mut found = None;
            for (i, c) in src[body_open..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            found = Some(body_open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            found.expect("function body must have a matching closing brace")
        };
        let body = &src[body_open..body_end];

        assert!(body.contains("lindera::prewarm()"));
        assert!(body.contains("tantivy_jieba::JiebaTokenizer"));
        assert!(body.contains("token_stream"));
        assert!(body.contains("chinese_convert::prewarm()"));
    }
}
