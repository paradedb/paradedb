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
use crate::api::tokenizers::typmod::validation::{PropertyRule, ValueConstraint};
use crate::api::tokenizers::typmod::{load_typmod, ParsedTypmod, TypmodSchema};
use tokenizers::manager::{LinderaLanguage, SearchTokenizerFilters};
use tokenizers::SearchNormalizer;

pub struct AliasTypmod {
    parsed: ParsedTypmod,
    filters: SearchTokenizerFilters,
}

pub struct UncheckedTypmod {
    parsed: ParsedTypmod,
    filters: SearchTokenizerFilters,
}

pub struct GenericTypmod {
    parsed: ParsedTypmod,
    pub filters: SearchTokenizerFilters,
}

pub struct NgramTypmod {
    pub min_gram: usize,
    pub max_gram: usize,
    pub prefix_only: bool,
    pub filters: SearchTokenizerFilters,
}

pub struct RegexTypmod {
    pub pattern: regex::Regex,
    pub filters: SearchTokenizerFilters,
}

pub struct LinderaTypmod {
    pub language: LinderaLanguage,
    pub filters: SearchTokenizerFilters,
}

pub struct UnicodeWordsTypmod {
    pub remove_emojis: bool,
    pub filters: SearchTokenizerFilters,
}

trait Typmod {
    fn schema() -> TypmodSchema;

    fn parsed(typmod: i32) -> typmod::Result<ParsedTypmod> {
        let parsed = load_typmod(typmod)?;
        Self::schema().validate(&parsed)?;
        Ok(parsed)
    }
}

impl Typmod for GenericTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![])
    }
}

impl Typmod for NgramTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![
            PropertyRule::new(
                "min",
                ValueConstraint::Integer {
                    min: Some(1),
                    max: None,
                },
            )
            .required()
            .positional(0),
            PropertyRule::new(
                "max",
                ValueConstraint::Integer {
                    min: Some(1),
                    max: None,
                },
            )
            .required()
            .positional(1),
            PropertyRule::new("prefix_only", ValueConstraint::Boolean),
        ])
    }
}

impl Typmod for RegexTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![PropertyRule::new("pattern", ValueConstraint::Regex)
            .required()
            .positional(0)])
    }
}

impl Typmod for LinderaTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![PropertyRule::new(
            "language",
            ValueConstraint::StringChoice(vec!["chinese", "japanese", "korean"]),
        )
        .required()
        .positional(0)])
    }
}

impl Typmod for UnicodeWordsTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![PropertyRule::new(
            "remove_emojis",
            ValueConstraint::Boolean,
        )
        .positional(0)])
    }
}

impl Typmod for AliasTypmod {
    fn schema() -> TypmodSchema {
        TypmodSchema::new(vec![PropertyRule::new("alias", ValueConstraint::String)
            .required()
            .positional(0)])
    }
}

impl TryFrom<i32> for GenericTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
        let filters = SearchTokenizerFilters::from(&parsed);
        Ok(GenericTypmod { parsed, filters })
    }
}

impl TryFrom<i32> for NgramTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
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
}

impl TryFrom<i32> for RegexTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
        let filters = SearchTokenizerFilters::from(&parsed);
        let pattern = parsed
            .try_get("pattern", 0)
            .and_then(|p| p.as_regex())
            .ok_or(typmod::Error::MissingKey("pattern"))??;

        Ok(RegexTypmod { pattern, filters })
    }
}

impl TryFrom<i32> for LinderaTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
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
}

impl TryFrom<i32> for UnicodeWordsTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
        let filters = SearchTokenizerFilters::from(&parsed);
        let remove_emojis = parsed.try_get("remove_emojis", 0).is_some();
        Ok(UnicodeWordsTypmod {
            remove_emojis,
            filters,
        })
    }
}

impl TryFrom<i32> for UncheckedTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = load_typmod(typmod)?;
        let filters = SearchTokenizerFilters::from(&parsed);
        Ok(UncheckedTypmod { parsed, filters })
    }
}

impl TryFrom<i32> for AliasTypmod {
    type Error = typmod::Error;
    fn try_from(typmod: i32) -> Result<Self, Self::Error> {
        let parsed = Self::parsed(typmod)?;
        let filters = SearchTokenizerFilters::from(&parsed);
        Ok(AliasTypmod { parsed, filters })
    }
}

impl UncheckedTypmod {
    pub fn alias(&self) -> Option<String> {
        self.parsed
            .get("alias")
            .map(|p| p.as_str().unwrap().to_string())
    }

    pub fn normalizer(&self) -> Option<SearchNormalizer> {
        self.filters.normalizer
    }
}

impl AliasTypmod {
    pub fn alias(&self) -> Option<String> {
        self.parsed
            .try_get("alias", 0)
            .and_then(|p| p.as_str())
            .map(|s| s.to_string())
    }
}
