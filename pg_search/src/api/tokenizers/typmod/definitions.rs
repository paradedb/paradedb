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

use crate::api::tokenizers::typmod;
use crate::api::tokenizers::typmod::{load_typmod, ParsedTypmod};
use tokenizers::manager::{LinderaLanguage, SearchTokenizerFilters};

pub struct GenericTypmod {
    parsed: ParsedTypmod,
    pub filters: SearchTokenizerFilters,
}

impl GenericTypmod {
    pub fn alias(&self) -> Option<String> {
        self.parsed
            .get("alias")
            .map(|p| p.as_str().unwrap().to_string())
    }
}

pub fn lookup_generic_typmod(typmod: i32) -> typmod::Result<GenericTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);

    Ok(GenericTypmod { parsed, filters })
}

pub struct NgramTypmod {
    pub min_gram: usize,
    pub max_gram: usize,
    pub prefix_only: bool,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_ngram_typmod(typmod: i32) -> typmod::Result<NgramTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);

    let min_gram = parsed
        .try_get("min", 0)
        .and_then(|p| p.as_usize())
        .ok_or(typmod::Error::MissingKey("min"))?;
    let max_gram = parsed
        .try_get("max", 1)
        .and_then(|p| p.as_usize())
        .ok_or(typmod::Error::MissingKey("max"))?;
    let prefix_only = parsed
        .get("prefix_only")
        .and_then(|p| p.as_bool())
        .unwrap_or(false);

    Ok(NgramTypmod {
        min_gram,
        max_gram,
        prefix_only,
        filters,
    })
}

pub struct RegexTypmod {
    pub pattern: regex::Regex,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_regex_typmod(typmod: i32) -> typmod::Result<RegexTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let pattern = parsed
        .try_get("pattern", 0)
        .and_then(|p| p.as_regex())
        .ok_or(typmod::Error::MissingKey("pattern"))??;

    Ok(RegexTypmod { pattern, filters })
}

pub struct LinderaTypmod {
    pub language: LinderaLanguage,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_lindera_typmod(typmod: i32) -> typmod::Result<LinderaTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let language = parsed
        .try_get("language", 0)
        .map(|p| match p.as_str() {
            None => panic!("missing language"),
            Some(s) => {
                let lcase = s.to_lowercase();
                match lcase.as_str() {
                    "chinese" => LinderaLanguage::Chinese,
                    "japanese" => LinderaLanguage::Japanese,
                    "korean" => LinderaLanguage::Korean,
                    other => panic!("unknown lindera language: {other}"),
                }
            }
        })
        .ok_or(typmod::Error::MissingKey("language"))?;
    Ok(LinderaTypmod { language, filters })
}

pub struct AliasTypmod {
    pub alias: String,
}

pub fn lookup_alias_typmod(typmod: i32) -> typmod::Result<AliasTypmod> {
    let parsed = load_typmod(typmod)?;
    let alias = parsed
        .try_get("alias", 0)
        .ok_or(typmod::Error::MissingKey("alias"))?;
    Ok(AliasTypmod {
        alias: alias.as_str().unwrap().to_string(),
    })
}

pub struct UnicodeWordsTypmod {
    pub remove_emojis: bool,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_unicode_words_typmod(typmod: i32) -> typmod::Result<UnicodeWordsTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let remove_emojis = parsed.try_get("remove_emojis", 0).is_some();
    Ok(UnicodeWordsTypmod {
        remove_emojis,
        filters,
    })
}
