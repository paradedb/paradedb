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

use crate::api::tokenizers::typmod::{ParsedTypmod, Property};
use crate::api::HashSet;
use std::sync::OnceLock;
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
                                key: key.unwrap_or(&prop.to_string()).to_string(),
                                message: format!("must be >= {min_val}, got {i}"),
                            });
                        }
                    }
                    if let Some(max_val) = max {
                        if i > *max_val {
                            return Err(ValidationError::InvalidValue {
                                key: key.unwrap_or(&prop.to_string()).to_string(),
                                message: format!("must be <= {max_val}, got {i}"),
                            });
                        }
                    }
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
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
                    actual_type: prop.to_string(),
                })
            }
            ValueConstraint::String => {
                if prop.as_str().is_some() {
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
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
                            key: key.unwrap_or(&prop.to_string()).to_string(),
                            message: format!(
                                "must be one of: [{}], got '{}'",
                                format_allowed_keys(allowed),
                                s
                            ),
                        })
                    }
                } else {
                    Err(ValidationError::TypeMismatch {
                        actual_type: prop.to_string(),
                    })
                }
            }
            ValueConstraint::Regex => {
                if prop.as_regex().is_some() {
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
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

macro_rules! rule {
    ($key:literal, $constraint:expr) => {
        PropertyRule::new($key, $constraint)
    };
    ($key:literal, $constraint:expr, required) => {
        PropertyRule::new($key, $constraint).required()
    };
    ($key:literal, $constraint:expr, positional = $pos:expr) => {
        PropertyRule::new($key, $constraint).positional($pos)
    };
    ($key:literal, $constraint:expr, required, positional = $pos:expr) => {
        PropertyRule::new($key, $constraint)
            .required()
            .positional($pos)
    };
    ($key:literal, $constraint:expr, positional = $pos:expr, required) => {
        PropertyRule::new($key, $constraint)
            .required()
            .positional($pos)
    };
}

pub(crate) use rule;

#[derive(Debug, Clone)]
pub struct TypmodSchema {
    rules: Vec<PropertyRule>,
}

impl TypmodSchema {
    pub fn new(rules: Vec<PropertyRule>) -> Self {
        static SHARED_RULES: OnceLock<Vec<PropertyRule>> = OnceLock::new();

        let shared_rules = SHARED_RULES.get_or_init(|| {
            vec![
                rule!(
                    "remove_short",
                    ValueConstraint::Integer {
                        min: Some(1),
                        max: None
                    }
                ),
                rule!(
                    "remove_long",
                    ValueConstraint::Integer {
                        min: Some(1),
                        max: None
                    }
                ),
                rule!("lowercase", ValueConstraint::Boolean),
                rule!(
                    "stemmer",
                    ValueConstraint::StringChoice(LANGUAGES.values().cloned().collect())
                ),
                rule!(
                    "stopwords_language",
                    ValueConstraint::StringChoice(LANGUAGES.values().cloned().collect())
                ),
                rule!("stopwords", ValueConstraint::String),
                rule!("alpha_num_only", ValueConstraint::Boolean),
                rule!("ascii_folding", ValueConstraint::Boolean),
                rule!("trim", ValueConstraint::Boolean),
                rule!(
                    "normalizer",
                    ValueConstraint::StringChoice(vec!["raw", "lowercase"])
                ),
                rule!("alias", ValueConstraint::String),
                rule!(
                    "chinese_convert",
                    ValueConstraint::StringChoice(vec![
                        "t2s", "s2t", "tw2s", "tw2sp", "s2tw", "s2twp"
                    ])
                ),
            ]
        });

        Self {
            rules: rules
                .into_iter()
                .chain(shared_rules.iter().cloned())
                .collect(),
        }
    }

    pub fn validate(&self, parsed: &ParsedTypmod) -> Result<(), ValidationError> {
        let allowed_keys: HashSet<&str> = self.rules.iter().map(|r| r.key).collect();
        let mut seen_keys: HashSet<&str> = HashSet::default();

        // validate provided typmod properties
        for (idx, prop) in parsed.properties.iter().enumerate() {
            if let Some(key) = prop.key() {
                if !self.rules.iter().any(|r| r.key == key) {
                    return Err(ValidationError::InvalidKey(
                        key.to_string(),
                        format_allowed_keys(&allowed_keys.iter().copied().collect::<Vec<_>>()),
                    ));
                }
                seen_keys.insert(key);

                if let Some(rule) = self.rules.iter().find(|r| r.key == key) {
                    rule.constraint.validate(prop, Some(key))?;
                }
            } else if let Some(rule) = self.rules.iter().find(|r| r.positional_index == Some(idx)) {
                rule.constraint.validate(prop, Some(rule.key))?;
                seen_keys.insert(rule.key);
            } else {
                return Err(ValidationError::NotAllowedAtPosition(prop.clone(), idx));
            }
        }

        // check that all required properties were provided
        for rule in &self.rules {
            if rule.required && !seen_keys.contains(rule.key) {
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

fn format_allowed_keys(keys: &[&str]) -> String {
    let mut sorted_keys = keys.to_owned();
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

    #[error("Cannot parse value of '{actual_type}'")]
    TypeMismatch { actual_type: String },
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
