use crate::query::value_to_json_term;
use crate::schema::IndexRecordOption;
use anyhow::Result;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::ops::Bound;
use tantivy::{
    query::{RangeQuery, RegexQuery, TermQuery},
    schema::{Field, OwnedValue},
    Term,
};

const EMPTY_KEY: &str = "empty";
const LOWER_KEY: &str = "lower";
const UPPER_KEY: &str = "upper";
const LOWER_INCLUSIVE_KEY: &str = "lower_inclusive";
const UPPER_INCLUSIVE_KEY: &str = "upper_inclusive";
const LOWER_UNBOUNDED_KEY: &str = "lower_unbounded";
const UPPER_UNBOUNDED_KEY: &str = "upper_unbounded";
// Always false for range fields
const EXPAND_DOTS: bool = false;
const RECORD: IndexRecordOption = IndexRecordOption::WithFreqsAndPositions;

#[derive(Clone, Debug)]
pub struct RangeField {
    field: Field,
    is_datetime: bool,
}

#[derive(Debug, PartialEq)]
pub enum Comparison {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl RangeField {
    pub fn new(field: Field, is_datetime: bool) -> Self {
        Self { field, is_datetime }
    }

    pub fn exists(&self) -> Result<RegexQuery, Box<dyn std::error::Error>> {
        Ok(RegexQuery::from_pattern(".*", self.field)?)
    }

    pub fn empty(&self, val: bool) -> Result<TermQuery, Box<dyn std::error::Error>> {
        let term = Self::as_range_term(self, &OwnedValue::Bool(val), Some(EMPTY_KEY))?;
        Ok(TermQuery::new(term, RECORD.into()))
    }

    pub fn upper_bound_inclusive(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        let term = Self::as_range_term(self, &OwnedValue::Bool(val), Some(UPPER_INCLUSIVE_KEY))?;
        Ok(TermQuery::new(term, RECORD.into()))
    }

    pub fn lower_bound_inclusive(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        let term = Self::as_range_term(self, &OwnedValue::Bool(val), Some(LOWER_INCLUSIVE_KEY))?;
        Ok(TermQuery::new(term, RECORD.into()))
    }

    pub fn upper_bound_unbounded(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        let term = Self::as_range_term(self, &OwnedValue::Bool(val), Some(UPPER_UNBOUNDED_KEY))?;
        Ok(TermQuery::new(term, RECORD.into()))
    }

    pub fn lower_bound_unbounded(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        let term = Self::as_range_term(self, &OwnedValue::Bool(val), Some(LOWER_UNBOUNDED_KEY))?;
        Ok(TermQuery::new(term, RECORD.into()))
    }

    pub fn compare_lower_bound(
        &self,
        owned: &OwnedValue,
        comparison: Comparison,
    ) -> Result<RangeQuery, Box<dyn std::error::Error>> {
        let query = match comparison {
            Comparison::LessThan => RangeQuery::new(
                Bound::Excluded(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::LessThanOrEqual => RangeQuery::new(
                Bound::Included(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::GreaterThan => RangeQuery::new(
                Bound::Unbounded,
                Bound::Excluded(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
            ),
            Comparison::GreaterThanOrEqual => RangeQuery::new(
                Bound::Unbounded,
                Bound::Included(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
            ),
        };

        Ok(query)
    }

    pub fn compare_upper_bound(
        &self,
        owned: &OwnedValue,
        comparison: Comparison,
    ) -> Result<RangeQuery, Box<dyn std::error::Error>> {
        let query = match comparison {
            Comparison::LessThan => RangeQuery::new(
                Bound::Excluded(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::LessThanOrEqual => RangeQuery::new(
                Bound::Included(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::GreaterThan => RangeQuery::new(
                Bound::Unbounded,
                Bound::Excluded(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
            ),
            Comparison::GreaterThanOrEqual => RangeQuery::new(
                Bound::Unbounded,
                Bound::Included(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
            ),
        };

        Ok(query)
    }

    fn as_range_term(
        &self,
        value: &OwnedValue,
        path: Option<&str>,
    ) -> Result<Term, Box<dyn std::error::Error>> {
        value_to_json_term(self.field, value, path, EXPAND_DOTS, self.is_datetime)
    }
}

/// Custom serialization function for `Bound<T>`.
/// The goal of this function is to serialize `Bound<T>` with **lowercase keys**.
/// By default, Rust would serialize the `Bound` enum using its variant names,
/// but we want to control the output format to ensure that keys like "included",
/// "excluded", and "unbounded" appear in lowercase.
pub fn serialize_bound<S, T>(bound: &Bound<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    match bound {
        Bound::Included(val) => {
            #[derive(Serialize)]
            #[serde(rename_all = "snake_case")]
            struct IncludedBound<T> {
                included: T,
            }
            IncludedBound { included: val }.serialize(serializer)
        }
        Bound::Excluded(val) => {
            #[derive(Serialize)]
            #[serde(rename_all = "snake_case")]
            struct ExcludedBound<T> {
                excluded: T,
            }
            ExcludedBound { excluded: val }.serialize(serializer)
        }
        Bound::Unbounded => {
            #[derive(Serialize)]
            #[serde(rename_all = "snake_case")]
            struct UnboundedBound;

            UnboundedBound.serialize(serializer)
        }
    }
}

/// Custom deserialization function for `Bound<T>`.
/// This function attempts to deserialize `Bound<T>` with lowercase keys (e.g., "included", "excluded"),
/// and if that fails, it falls back to deserializing with capitalized keys ("Included", "Excluded").
// pub fn deserialize_bound<'de, D, T>(deserializer: D) -> Result<Bound<T>, D::Error>
// where
//     D: Deserializer<'de>,
//     T: Deserialize<'de>,
// {
//     // First, deserialize into a `serde_json::Value`.
//     let value: Value = Value::deserialize(deserializer)?;

//     // Try to deserialize using lowercase keys.
//     if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
//         return match bound {
//             LowercaseBoundDef::Included { included } => Ok(Bound::Included(included)),
//             LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Excluded(excluded)),
//             LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
//         };
//     }

//     // If lowercase deserialization fails, try with capitalized keys.
//     let bound = CapitalizedBoundDef::deserialize(value)
//         .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

//     match bound {
//         CapitalizedBoundDef::Included { Included } => Ok(Bound::Included(Included)),
//         CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Excluded(Excluded)),
//         CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
//     }
// }

pub fn deserialize_tantivy_lower_bound<'de, D>(
    deserializer: D,
) -> Result<Bound<OwnedValue>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, deserialize into a `serde_json::Value`.
    let value: Value = Value::deserialize(deserializer)?;

    // Try to deserialize using lowercase keys.
    if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
        return match bound {
            LowercaseBoundDef::Included { included } => Ok(Bound::Included(included)),
            LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Included(match excluded {
                OwnedValue::U64(i) => OwnedValue::U64(i + 1),
                OwnedValue::I64(i) => OwnedValue::I64(i + 1),
                _ => excluded,
            })),
            LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
        };
    }

    // If lowercase deserialization fails, try with capitalized keys.
    let bound = CapitalizedBoundDef::deserialize(value)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

    match bound {
        CapitalizedBoundDef::Included { Included } => Ok(Bound::Included(Included)),
        CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Included(match Excluded {
            OwnedValue::U64(i) => OwnedValue::U64(i + 1),
            OwnedValue::I64(i) => OwnedValue::I64(i + 1),
            _ => Excluded,
        })),
        CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
    }
}

pub fn deserialize_u64_lower_bound<'de, D>(deserializer: D) -> Result<Bound<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, deserialize into a `serde_json::Value`.
    let value: Value = Value::deserialize(deserializer)?;

    // Try to deserialize using lowercase keys.
    if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
        return match bound {
            LowercaseBoundDef::Included { included } => Ok(Bound::Included(included)),
            LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Included(excluded + 1)),
            LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
        };
    }

    // If lowercase deserialization fails, try with capitalized keys.
    let bound = CapitalizedBoundDef::deserialize(value)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

    match bound {
        CapitalizedBoundDef::Included { Included } => Ok(Bound::Included(Included)),
        CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Included(Excluded + 1)),
        CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
    }
}

pub fn deserialize_tantivy_upper_bound<'de, D>(
    deserializer: D,
) -> Result<Bound<OwnedValue>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, deserialize into a `serde_json::Value`.
    let value: Value = Value::deserialize(deserializer)?;

    // Try to deserialize using lowercase keys.
    if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
        return match bound {
            LowercaseBoundDef::Included { included } => Ok(Bound::Excluded(match included {
                OwnedValue::U64(i) => OwnedValue::U64(i + 1),
                OwnedValue::I64(i) => OwnedValue::I64(i + 1),
                _ => included,
            })),
            LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Excluded(excluded)),
            LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
        };
    }

    // If lowercase deserialization fails, try with capitalized keys.
    let bound = CapitalizedBoundDef::deserialize(value)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

    match bound {
        CapitalizedBoundDef::Included { Included } => Ok(Bound::Excluded(match Included {
            OwnedValue::U64(i) => OwnedValue::U64(i + 1),
            OwnedValue::I64(i) => OwnedValue::I64(i + 1),
            _ => Included,
        })),
        CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Excluded(Excluded)),
        CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
    }
}

pub fn deserialize_u64_upper_bound<'de, D>(deserializer: D) -> Result<Bound<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, deserialize into a `serde_json::Value`.
    let value: Value = Value::deserialize(deserializer)?;

    // Try to deserialize using lowercase keys.
    if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
        return match bound {
            LowercaseBoundDef::Included { included } => Ok(Bound::Excluded(included + 1)),
            LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Excluded(excluded)),
            LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
        };
    }

    // If lowercase deserialization fails, try with capitalized keys.
    let bound = CapitalizedBoundDef::deserialize(value)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

    match bound {
        CapitalizedBoundDef::Included { Included } => Ok(Bound::Excluded(Included + 1)),
        CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Excluded(Excluded)),
        CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
    }
}

// Define Lowercase and Capitalized variants to support both cases.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
enum LowercaseBoundDef<T> {
    Included { included: T },
    Excluded { excluded: T },
    Unbounded,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
#[serde(untagged)]
#[allow(non_snake_case)]
enum CapitalizedBoundDef<T> {
    Included { Included: T },
    Excluded { Excluded: T },
    Unbounded,
}
