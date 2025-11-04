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

use crate::api::tokenizers::typmod::{ParsedTypmod, Property};
use std::collections::HashSet;
use thiserror::Error;
use tokenizers::manager::LANGUAGES;

#[derive(Debug, Clone)]
pub enum ValueConstraint {
    Integer { min: Option<i64>, max: Option<i64> },
    Boolean,
    String,
    StringChoice(Vec<&'static str>),
    Regex,
}

impl ValueConstraint {
    fn validate(&self, prop: &Property, key: Option<&str>) -> Result<(), ValidationError> {
        match self {
            ValueConstraint::Integer { min, max } => {
                if let Some(i) = prop.as_usize().map(|v| v as i64) {
                    if let Some(min_val) = min {
                        if i < *min_val {
                            return Err(ValidationError::InvalidValue {
                                key: key.unwrap_or("<positional>").to_string(),
                                message: format!("must be >= {min_val}, got {i}"),
                            });
                        }
                    }
                    if let Some(max_val) = max {
                        if i > *max_val {
                            return Err(ValidationError::InvalidValue {
                                key: key.unwrap_or("<positional>").to_string(),
                                message: format!("must be <= {max_val}, got {i}"),
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
                        key: key.unwrap_or("<positional>").to_string(),
                        expected_type: "integer".to_string(),
                        actual_type: prop.to_string(),
                    })
                }
            }
            ValueConstraint::Boolean => {
                if prop.as_bool().is_some() {
                    return Ok(());
                } else if let (Some(expected_key), Property::String(None, value)) = (key, prop) {
                    if value == expected_key {
                        return Ok(());
                    }
                }

                Err(ValidationError::TypeMismatch {
                    key: key.unwrap_or("<positional>").to_string(),
                    expected_type: "boolean".to_string(),
                    actual_type: prop.to_string(),
                })
            }
            ValueConstraint::String => {
                if prop.as_str().is_some() {
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
                        key: key.unwrap_or("<positional>").to_string(),
                        expected_type: "string".to_string(),
                        actual_type: prop.to_string(),
                    })
                }
            }
            ValueConstraint::StringChoice(allowed) => {
                if let Some(s) = prop.as_str() {
                    let lcase = s.to_lowercase();
                    if allowed
                        .iter()
                        .any(|allowed_val| allowed_val.to_lowercase() == lcase)
                    {
                        Ok(())
                    } else {
                        Err(ValidationError::InvalidValue {
                            key: key.unwrap_or("<positional>").to_string(),
                            message: format!("must be one of: {}, got '{}'", allowed.join(", "), s),
                        })
                    }
                } else {
                    Err(ValidationError::TypeMismatch {
                        key: key.unwrap_or("<positional>").to_string(),
                        expected_type: "string".to_string(),
                        actual_type: prop.to_string(),
                    })
                }
            }
            ValueConstraint::Regex => {
                if prop.as_regex().is_some() {
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
                        key: key.unwrap_or("<positional>").to_string(),
                        expected_type: "regex".to_string(),
                        actual_type: prop.to_string(),
                    })
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyRule {
    pub key: &'static str,
    pub required: bool,
    pub constraint: ValueConstraint,
    pub positional_index: Option<usize>,
}

impl PropertyRule {
    pub fn new(key: &'static str, constraint: ValueConstraint) -> Self {
        Self {
            key,
            required: false,
            constraint,
            positional_index: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn positional(mut self, index: usize) -> Self {
        self.positional_index = Some(index);
        self
    }
}

/// Defines the validation schema for a tokenizer typmod
#[derive(Debug, Clone)]
pub struct TypmodSchema {
    rules: Vec<PropertyRule>,
}

impl TypmodSchema {
    pub fn new(rules: Vec<PropertyRule>) -> Self {
        let search_tokenizer_filters_rules: [PropertyRule; 9] = [
            PropertyRule::new(
                "remove_short",
                ValueConstraint::Integer {
                    min: Some(1),
                    max: None,
                },
            ),
            PropertyRule::new(
                "remove_long",
                ValueConstraint::Integer {
                    min: Some(1),
                    max: None,
                },
            ),
            PropertyRule::new("lowercase", ValueConstraint::Boolean),
            PropertyRule::new(
                "stemmer",
                ValueConstraint::StringChoice(LANGUAGES.values().cloned().collect()),
            ),
            PropertyRule::new(
                "stopwords_language",
                ValueConstraint::StringChoice(LANGUAGES.values().cloned().collect()),
            ),
            PropertyRule::new("stopwords", ValueConstraint::String),
            PropertyRule::new("alpha_num_only", ValueConstraint::Boolean),
            PropertyRule::new("ascii_folding", ValueConstraint::Boolean),
            PropertyRule::new(
                "normalizer",
                ValueConstraint::StringChoice(vec!["raw", "lowercase"]),
            ),
        ];

        let alias_rules: [PropertyRule; 1] = [PropertyRule::new("alias", ValueConstraint::String)];

        let rules = [
            rules.as_slice(),
            &search_tokenizer_filters_rules,
            &alias_rules,
        ]
        .concat();
        Self { rules }
    }

    /// Validates a ParsedTypmod against this schema
    pub fn validate(&self, parsed: &ParsedTypmod) -> Result<(), ValidationError> {
        let allowed_keys: HashSet<String> = self.rules.iter().map(|r| r.key.to_string()).collect();
        let mut seen_keys: HashSet<String> = HashSet::new();

        // validate all properties
        for (idx, prop) in parsed.properties.iter().enumerate() {
            if let Some(key) = prop.key() {
                if !allowed_keys.contains(key) {
                    return Err(ValidationError::InvalidKey(
                        key.to_string(),
                        format_allowed_keys(&allowed_keys),
                    ));
                }
                seen_keys.insert(key.to_string());

                if let Some(rule) = self.rules.iter().find(|r| r.key == key) {
                    rule.constraint.validate(prop, Some(key))?;
                }
            } else if let Some(rule) = self.rules.iter().find(|r| r.positional_index == Some(idx)) {
                rule.constraint.validate(prop, Some(rule.key))?;
                seen_keys.insert(rule.key.to_string());
            } else {
                return Err(ValidationError::NotAllowedAtPosition(prop.clone(), idx));
            }
        }

        // check for missing required keys
        for rule in &self.rules {
            if rule.required && !seen_keys.contains(rule.key) {
                // check if it's a positional property - if so, it should have been seen above
                if let Some(pos_idx) = rule.positional_index {
                    if pos_idx >= parsed.properties.len() {
                        return Err(ValidationError::MissingRequiredKey(rule.key.to_string()));
                    }
                }
                return Err(ValidationError::MissingRequiredKey(rule.key.to_string()));
            }
        }

        Ok(())
    }
}

fn format_allowed_keys(keys: &HashSet<String>) -> String {
    let mut sorted_keys: Vec<String> = keys.iter().cloned().collect();
    sorted_keys.sort();
    sorted_keys.join(", ")
}

#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Invalid option: '{0}'. Allowed options: {1}.")]
    InvalidKey(String, String),

    #[error("Missing required option: '{0}'")]
    MissingRequiredKey(String),

    #[error("Invalid value for option '{key}': {message}")]
    InvalidValue { key: String, message: String },

    #[error("Option '{0}' is not allowed at position {1}")]
    NotAllowedAtPosition(Property, usize),

    #[error("Value for option '{key}' must be of type {expected_type}, got {actual_type}")]
    TypeMismatch {
        key: String,
        expected_type: String,
        actual_type: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::tokenizers::typmod::ParsedTypmod;

    #[test]
    fn test_validate_integer_range() {
        let schema = TypmodSchema::new(vec![PropertyRule::new(
            "min",
            ValueConstraint::Integer {
                min: Some(1),
                max: Some(10),
            },
        )]);

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::Integer(Some("min".to_string()), 5));
        assert!(schema.validate(&parsed).is_ok());

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::Integer(Some("min".to_string()), 0));
        assert!(schema.validate(&parsed).is_err());

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::Integer(Some("min".to_string()), 15));
        assert!(schema.validate(&parsed).is_err());
    }

    #[test]
    fn test_validate_string_choice() {
        let schema = TypmodSchema::new(vec![PropertyRule::new(
            "language",
            ValueConstraint::StringChoice(vec!["chinese", "japanese", "korean"]),
        )]);

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::String(
            Some("language".to_string()),
            "chinese".to_string(),
        ));
        assert!(schema.validate(&parsed).is_ok());

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::String(
            Some("language".to_string()),
            "french".to_string(),
        ));
        assert!(schema.validate(&parsed).is_err());
    }

    #[test]
    fn test_validate_required_key() {
        let schema = TypmodSchema::new(vec![PropertyRule::new(
            "required_key",
            ValueConstraint::String,
        )
        .required()]);

        let parsed = ParsedTypmod::new();
        assert!(schema.validate(&parsed).is_err());

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::String(
            Some("required_key".to_string()),
            "value".to_string(),
        ));
        assert!(schema.validate(&parsed).is_ok());
    }

    #[test]
    fn test_validate_invalid_key() {
        let schema = TypmodSchema::new(vec![]);

        let mut parsed = ParsedTypmod::new();
        parsed.add_property(Property::String(
            Some("invalid_key".to_string()),
            "value".to_string(),
        ));
        assert!(schema.validate(&parsed).is_err());
    }
}
