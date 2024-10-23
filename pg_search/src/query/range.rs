use crate::query::value_to_json_term;
use crate::schema::IndexRecordOption;
use anyhow::{anyhow, Result};
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::ops::Bound;
use tantivy::DateTime;
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
#[allow(dead_code)] // improperly reported as dead code
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
pub fn deserialize_bound<'de, D, T>(deserializer: D) -> Result<Bound<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // First, deserialize into a `serde_json::Value`.
    let value: Value = Value::deserialize(deserializer)?;
    if value.as_str() == Some("Unbounded") {
        return Ok(Bound::Unbounded);
    }

    pgrx::log!("bound: {value:?}");
    // Try to deserialize using lowercase keys.
    if let Ok(bound) = LowercaseBoundDef::deserialize(value.clone()) {
        return match bound {
            LowercaseBoundDef::Included { included } => Ok(Bound::Included(included)),
            LowercaseBoundDef::Excluded { excluded } => Ok(Bound::Excluded(excluded)),
            LowercaseBoundDef::Unbounded => Ok(Bound::Unbounded),
        };
    }

    // If lowercase deserialization fails, try with capitalized keys.
    let bound = CapitalizedBoundDef::deserialize(value)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize: {}", e)))?; // Convert serde_json error to D::Error

    match bound {
        CapitalizedBoundDef::Included { Included } => Ok(Bound::Included(Included)),
        CapitalizedBoundDef::Excluded { Excluded } => Ok(Bound::Excluded(Excluded)),
        CapitalizedBoundDef::Unbounded => Ok(Bound::Unbounded),
    }
}

pub fn deserialize_date_bound(bound: &Bound<OwnedValue>) -> Result<Bound<OwnedValue>> {
    match bound {
        Bound::Included(OwnedValue::Str(date_string)) => {
            let date: chrono::DateTime<chrono::Utc> = date_string
                .parse()
                .expect("included date string must parse to date");
            let nanos = date
                .timestamp_nanos_opt()
                .expect("included date string must not overflow");
            Ok(Bound::Included(OwnedValue::Date(
                DateTime::from_timestamp_nanos(nanos),
            )))
        }
        Bound::Excluded(OwnedValue::Str(date_string)) => {
            let date: chrono::DateTime<chrono::Utc> = date_string
                .parse()
                .expect("excluded date string must parse to date");
            let nanos = date
                .timestamp_nanos_opt()
                .expect("excluded date string must not overflow");
            Ok(Bound::Excluded(OwnedValue::Date(
                DateTime::from_timestamp_nanos(nanos),
            )))
        }
        Bound::Unbounded => Ok(Bound::Unbounded),
        other => Err(anyhow!("value must be a string, received: {other:?}")),
    }
}

pub fn canonicalize_tantivy_lower_bound(bound: &Bound<OwnedValue>) -> Bound<OwnedValue> {
    let one_day_nanos: i64 = 86_400_000_000_000;
    match bound {
        std::ops::Bound::Excluded(excluded) => std::ops::Bound::Included(match excluded {
            OwnedValue::U64(i) => OwnedValue::U64(i + 1),
            OwnedValue::I64(i) => OwnedValue::I64(i + 1),
            OwnedValue::Date(date) => OwnedValue::Date(DateTime::from_timestamp_nanos(
                date.into_timestamp_nanos() + one_day_nanos,
            )),
            _ => excluded.clone(),
        }),
        other => other.clone(),
    }
}

pub fn canonicalize_tantivy_upper_bound(bound: &Bound<OwnedValue>) -> Bound<OwnedValue> {
    let one_day_nanos: i64 = 86_400_000_000_000;
    match bound {
        std::ops::Bound::Included(included) => std::ops::Bound::Excluded(match included {
            OwnedValue::U64(i) => OwnedValue::U64(i + 1),
            OwnedValue::I64(i) => OwnedValue::I64(i + 1),
            OwnedValue::Date(date) => OwnedValue::Date(DateTime::from_timestamp_nanos(
                date.into_timestamp_nanos() + one_day_nanos,
            )),
            _ => included.clone(),
        }),
        other => other.clone(),
    }
}

pub fn canonicalize_lower_bound_u64(bound: &Bound<u64>) -> Bound<u64> {
    match bound {
        std::ops::Bound::Excluded(excluded) => std::ops::Bound::Included(excluded + 1),
        other => other.clone(),
    }
}

pub fn canonicalize_upper_bound_u64(bound: &Bound<u64>) -> Bound<u64> {
    match bound {
        std::ops::Bound::Included(included) => std::ops::Bound::Excluded(included + 1),
        other => other.clone(),
    }
}

pub fn deserialize_range<'de, D>(
    deserializer: D,
) -> Result<
    (
        String,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        bool,
    ),
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct RangeHelper {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    }
    let mut helper = RangeHelper::deserialize(deserializer)?;

    if helper.is_datetime {
        helper.lower_bound = deserialize_date_bound(&helper.lower_bound)
            .expect("must be able to deserialize date in lower_bound");
        helper.upper_bound = deserialize_date_bound(&helper.upper_bound)
            .expect("must be able to deserialize date in upper_bound");
    }

    helper.lower_bound = canonicalize_tantivy_lower_bound(&helper.lower_bound);
    helper.upper_bound = canonicalize_tantivy_upper_bound(&helper.upper_bound);

    Ok((
        helper.field,
        helper.lower_bound,
        helper.upper_bound,
        helper.is_datetime,
    ))
}

pub fn deserialize_range_contains<'de, D>(
    deserializer: D,
) -> Result<
    (
        String,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        bool,
    ),
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct RangeHelper {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    }

    let mut helper = RangeHelper::deserialize(deserializer)?;

    if helper.is_datetime {
        helper.lower_bound = deserialize_date_bound(&helper.lower_bound)
            .expect("must be able to deserialize date in lower_bound");
        helper.upper_bound = deserialize_date_bound(&helper.upper_bound)
            .expect("must be able to deserialize date in upper_bound");
    }

    helper.lower_bound = canonicalize_tantivy_lower_bound(&helper.lower_bound);
    helper.upper_bound = canonicalize_tantivy_upper_bound(&helper.upper_bound);

    Ok((
        helper.field,
        helper.lower_bound,
        helper.upper_bound,
        helper.is_datetime,
    ))
}

pub fn deserialize_range_intersects<'de, D>(
    deserializer: D,
) -> Result<
    (
        String,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        bool,
    ),
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct RangeHelper {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    }

    let mut helper = RangeHelper::deserialize(deserializer)?;

    if helper.is_datetime {
        helper.lower_bound = deserialize_date_bound(&helper.lower_bound)
            .expect("must be able to deserialize date in lower_bound");
        helper.upper_bound = deserialize_date_bound(&helper.upper_bound)
            .expect("must be able to deserialize date in upper_bound");
    }

    helper.lower_bound = canonicalize_tantivy_lower_bound(&helper.lower_bound);
    helper.upper_bound = canonicalize_tantivy_upper_bound(&helper.upper_bound);

    Ok((
        helper.field,
        helper.lower_bound,
        helper.upper_bound,
        helper.is_datetime,
    ))
}

pub fn deserialize_range_within<'de, D>(
    deserializer: D,
) -> Result<
    (
        String,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        std::ops::Bound<tantivy::schema::OwnedValue>,
        bool,
    ),
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct RangeHelper {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
        #[serde(default)]
        is_datetime: bool,
    }

    let mut helper = RangeHelper::deserialize(deserializer)?;

    if helper.is_datetime {
        helper.lower_bound = deserialize_date_bound(&helper.lower_bound)
            .expect("must be able to deserialize date in lower_bound");
        helper.upper_bound = deserialize_date_bound(&helper.upper_bound)
            .expect("must be able to deserialize date in upper_bound");
    }

    helper.lower_bound = canonicalize_tantivy_lower_bound(&helper.lower_bound);
    helper.upper_bound = canonicalize_tantivy_upper_bound(&helper.upper_bound);

    Ok((
        helper.field,
        helper.lower_bound,
        helper.upper_bound,
        helper.is_datetime,
    ))
}

pub fn deserialize_fast_field_range_weight<'de, D>(
    deserializer: D,
) -> Result<(String, std::ops::Bound<u64>, std::ops::Bound<u64>), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct FastFieldRangeHelper {
        field: String,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        lower_bound: std::ops::Bound<u64>,
        #[serde(
            serialize_with = "serialize_bound",
            deserialize_with = "deserialize_bound"
        )]
        upper_bound: std::ops::Bound<u64>,
    }

    let helper = FastFieldRangeHelper::deserialize(deserializer)?;

    Ok((
        helper.field,
        canonicalize_lower_bound_u64(&helper.lower_bound),
        canonicalize_upper_bound_u64(&helper.upper_bound),
    ))
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
