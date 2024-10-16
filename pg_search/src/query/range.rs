use crate::query::value_to_json_term;
use crate::schema::IndexRecordOption;
use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// Custom serialization function for `Bound<T>`.
/// The goal of this function is to deserialize `Bound<T>` with **lowercase keys**.
/// By default, Rust would deserialize the `Bound` enum using its variant names,
/// but we want to support a structure with keys like "included",
/// "excluded", and "unbounded" appearing in lowercase.
pub fn deserialize_bound<'de, D, T>(deserializer: D) -> Result<Bound<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[serde(untagged)]
    enum BoundDef<T> {
        Included { included: T },
        Excluded { excluded: T },
        Unbounded,
    }

    let bound_def = BoundDef::deserialize(deserializer)?;
    match bound_def {
        BoundDef::Included { included } => Ok(Bound::Included(included)),
        BoundDef::Excluded { excluded } => Ok(Bound::Excluded(excluded)),
        BoundDef::Unbounded { .. } => Ok(Bound::Unbounded),
    }
}
