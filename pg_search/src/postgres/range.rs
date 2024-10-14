use crate::postgres::types::{TantivyValue, TantivyValueError};
use crate::schema::range::TantivyRangeBuilder;
use pgrx::datum::{Date, DateTimeConversionError, RangeBound, Timestamp, TimestampWithTimeZone};
use serde::{Deserialize, Serialize};

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

pub(crate) trait RangeToTantivyValue<T, S>
where
    T: Serialize + pgrx::datum::RangeSubType,
    S: TryFrom<T> + Serialize + Clone,
    <S as TryFrom<T>>::Error: std::fmt::Debug,
{
    fn from_range(val: pgrx::Range<T>) -> Result<TantivyValue, TantivyValueError> {
        pgrx::info!("Converting range to Tantivy value");
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
impl RangeToTantivyValue<pgrx::AnyNumeric, f64> for TantivyValue {}
impl RangeToTantivyValue<Date, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<Timestamp, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<TimestampWithTimeZone, TimestampWithTimeZone> for TantivyValue {}
