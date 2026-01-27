use crate::postgres::types::{TantivyValue, TantivyValueError};
use crate::schema::range::TantivyRangeBuilder;
use decimal_bytes::Decimal;
use pgrx::datum::{Date, DateTimeConversionError, RangeBound, Timestamp, TimestampWithTimeZone};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

// When Tantivy reads JSON objects, it only recognizes RFC 3339 formatted strings as DateTime values.
// Dates like "2021-01-01" or ISO formatted strings like "2021-01-01T00:00:00" are not recognized.
// To work around this, we convert Date and Timestamp values to TimestampWithTimeZone values with the UTC timezone,
// which gets serialized to RFC 3339 ie "2021-01-01T00:00:00Z".
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct TimestampWithTimeZoneUtc(pub TimestampWithTimeZone);

impl TryFrom<Date> for TimestampWithTimeZoneUtc {
    type Error = DateTimeConversionError;

    fn try_from(val: Date) -> Result<Self, Self::Error> {
        let tstz = TimestampWithTimeZone::from(val);
        Ok(TimestampWithTimeZoneUtc(
            TimestampWithTimeZone::with_timezone(
                tstz.year(),
                tstz.month(),
                tstz.day(),
                0,
                0,
                0.0,
                "UTC",
            )?,
        ))
    }
}

impl TryFrom<Timestamp> for TimestampWithTimeZoneUtc {
    type Error = DateTimeConversionError;

    fn try_from(val: Timestamp) -> Result<Self, Self::Error> {
        let tstz = TimestampWithTimeZone::from(val);
        Ok(TimestampWithTimeZoneUtc(
            TimestampWithTimeZone::with_timezone(
                tstz.year(),
                tstz.month(),
                tstz.day(),
                tstz.hour(),
                tstz.minute(),
                tstz.second(),
                "UTC",
            )?,
        ))
    }
}

/// A wrapper around `Decimal` that serializes to hex-encoded lexicographically sortable bytes.
/// This allows string comparison in Tantivy's JSON fields to give correct numeric ordering.
///
/// Used for `numrange` to preserve full NUMERIC precision in range queries.
#[derive(Clone, Debug)]
pub(crate) struct SortableDecimal(pub Decimal);

impl TryFrom<pgrx::AnyNumeric> for SortableDecimal {
    type Error = decimal_bytes::DecimalError;

    fn try_from(val: pgrx::AnyNumeric) -> Result<Self, Self::Error> {
        let numeric_str = val.normalize().to_string();
        let decimal = Decimal::from_str(&numeric_str)?;
        Ok(SortableDecimal(decimal))
    }
}

impl Serialize for SortableDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as hex-encoded sortable bytes.
        // Hex encoding preserves lexicographic ordering since:
        // - Each byte maps to exactly 2 hex chars
        // - Hex chars compare in the same order as byte values
        let hex: String = self
            .0
            .as_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for SortableDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        // Decode hex string to bytes
        let bytes: Result<Vec<u8>, _> = (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
            .collect();
        let bytes = bytes.map_err(serde::de::Error::custom)?;
        let decimal = Decimal::from_bytes(&bytes).map_err(serde::de::Error::custom)?;
        Ok(SortableDecimal(decimal))
    }
}

pub(crate) trait RangeToTantivyValue<T, S>
where
    T: Serialize + pgrx::datum::RangeSubType,
    S: TryFrom<T> + Serialize + Clone,
    <S as TryFrom<T>>::Error: std::fmt::Debug,
{
    fn from_range(val: pgrx::Range<T>) -> Result<TantivyValue, TantivyValueError> {
        match val.is_empty() {
            true => Ok(TantivyValue(tantivy::schema::OwnedValue::from(
                serde_json::to_value(TantivyRangeBuilder::<T>::new().empty(true).build())?,
            ))),
            false => {
                let lower = match val.lower() {
                    Some(RangeBound::Inclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Exclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Infinite) | None => None,
                };
                let upper = match val.upper() {
                    Some(RangeBound::Inclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Exclusive(val)) => Some(S::try_from(val.clone()).unwrap()),
                    Some(RangeBound::Infinite) | None => None,
                };

                let lower_inclusive = matches!(val.lower(), Some(RangeBound::Inclusive(_)));
                let upper_inclusive = matches!(val.upper(), Some(RangeBound::Inclusive(_)));
                let lower_unbounded = matches!(val.lower(), Some(RangeBound::Infinite) | None);
                let upper_unbounded = matches!(val.upper(), Some(RangeBound::Infinite) | None);

                Ok(TantivyValue(tantivy::schema::OwnedValue::from(
                    serde_json::to_value(
                        TantivyRangeBuilder::new()
                            .lower(lower)
                            .upper(upper)
                            .lower_inclusive(lower_inclusive)
                            .upper_inclusive(upper_inclusive)
                            .lower_unbounded(lower_unbounded)
                            .upper_unbounded(upper_unbounded)
                            .build(),
                    )?,
                )))
            }
        }
    }
}

impl RangeToTantivyValue<i32, i32> for TantivyValue {}
impl RangeToTantivyValue<i64, i64> for TantivyValue {}
// numrange uses SortableDecimal which serializes as hex-encoded lexicographically sortable bytes.
// This preserves full NUMERIC precision while allowing string comparison to give correct ordering.
impl RangeToTantivyValue<pgrx::AnyNumeric, SortableDecimal> for TantivyValue {}
impl RangeToTantivyValue<Date, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<Timestamp, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<TimestampWithTimeZone, TimestampWithTimeZone> for TantivyValue {}
