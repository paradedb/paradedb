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

/// Register `search_tokenizers` into an existing manager. `TokenizerManager` registration
/// mutates its shared registry, so readers already holding the manager see the entries too.
pub fn register_tokenizers_into(
    tokenizer_manager: &TokenizerManager,
    search_tokenizers: Vec<SearchTokenizer>,
) {
    // Names this call itself has claimed -- current-name registrations, and any legacy aliases
    // already placed. `TokenizerManager::default()` pre-seeds built-ins ("raw", "default",
    // "whitespace", "en_stem") before this ever runs, and a `trim`-affected tokenizer's legacy
    // name can coincidentally match one of those (e.g. an empty-filter `pdb.whitespace` with
    // `trim=true` legacy-aliases to plain "whitespace"). Those built-ins aren't real claims from
    // this batch, so collision detection must check this set, not the manager's live state --
    // otherwise the alias gets skipped and the field silently falls back to Tantivy's raw
    // built-in analyzer (e.g. losing ParadeDB's default lowercasing) instead of its own.
    let mut claimed_names = std::collections::HashSet::new();

    // (legacy_name, current_name) pairs to alias once every tokenizer's current name is registered.
    let mut legacy_aliases = Vec::new();

    for search_tokenizer in &search_tokenizers {
        let Some(text_analyzer) = search_tokenizer.to_tantivy_tokenizer() else {
            continue;
        };

        let current_name = search_tokenizer.name();
        debug!(tokenizer_name = &current_name, "registering tokenizer");
        tokenizer_manager.register(&current_name, text_analyzer);
        claimed_names.insert(current_name.clone());

        if let Some(legacy_name) = search_tokenizer.legacy_name_before_trim_fix()
            && legacy_name != current_name
        {
            legacy_aliases.push((legacy_name, current_name));
        }
    }

    // Indexes built before the trim-filter naming fix have the pre-fix name baked into their
    // on-disk Tantivy schema, so that name must still resolve to the same analyzer. Register
    // these aliases only after every tokenizer's current name is in place, and only when no
    // other tokenizer in this batch already claims that name -- otherwise we'd silently
    // reintroduce the exact name collision this fix exists to prevent going forward.
    for (legacy_name, current_name) in legacy_aliases {
        if claimed_names.contains(&legacy_name) {
            continue;
        }
        if let Some(text_analyzer) = tokenizer_manager.get(&current_name) {
            debug!(
                tokenizer_name = &legacy_name,
                aliases = %current_name,
                "registering legacy pre-trim-fix tokenizer alias",
            );
            tokenizer_manager.register(&legacy_name, text_analyzer);
            claimed_names.insert(legacy_name);
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
