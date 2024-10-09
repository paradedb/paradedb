use crate::postgres::types::{TantivyValue, TantivyValueError};
use pgrx::datum::{Date, DateTimeConversionError, RangeBound, Timestamp, TimestampWithTimeZone};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TantivyRange<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_inclusive: bool,
    upper_inclusive: bool,
    lower_unbounded: bool,
    upper_unbounded: bool,
}

pub(crate) struct TantivyRangeBuilder<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_inclusive: Option<bool>,
    upper_inclusive: Option<bool>,
    lower_unbounded: Option<bool>,
    upper_unbounded: Option<bool>,
}

impl<T> TantivyRangeBuilder<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            lower: None,
            upper: None,
            lower_inclusive: None,
            upper_inclusive: None,
            lower_unbounded: None,
            upper_unbounded: None,
        }
    }

    pub fn lower(mut self, lower: Option<T>) -> Self {
        self.lower = lower;
        self
    }

    pub fn upper(mut self, upper: Option<T>) -> Self {
        self.upper = upper;
        self
    }

    pub fn lower_inclusive(mut self, lower_inclusive: bool) -> Self {
        self.lower_inclusive = Some(lower_inclusive);
        self
    }

    pub fn upper_inclusive(mut self, upper_inclusive: bool) -> Self {
        self.upper_inclusive = Some(upper_inclusive);
        self
    }

    pub fn lower_unbounded(mut self, lower_unbounded: bool) -> Self {
        self.lower_unbounded = Some(lower_unbounded);
        self
    }

    pub fn upper_unbounded(mut self, upper_unbounded: bool) -> Self {
        self.upper_unbounded = Some(upper_unbounded);
        self
    }

    pub fn build(self) -> TantivyRange<T> {
        TantivyRange {
            lower: self.lower,
            upper: self.upper,
            lower_inclusive: self.lower_inclusive.unwrap_or(true),
            upper_inclusive: self.upper_inclusive.unwrap_or(false),
            lower_unbounded: self.lower_unbounded.unwrap_or(false),
            upper_unbounded: self.upper_unbounded.unwrap_or(false),
        }
    }
}

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
        match val.is_empty() {
            true => Ok(TantivyValue(tantivy::schema::OwnedValue::from(
                serde_json::to_value(TantivyRangeBuilder::<T>::new().build())?,
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
impl RangeToTantivyValue<pgrx::AnyNumeric, pgrx::AnyNumeric> for TantivyValue {}
impl RangeToTantivyValue<Date, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<Timestamp, TimestampWithTimeZoneUtc> for TantivyValue {}
impl RangeToTantivyValue<TimestampWithTimeZone, TimestampWithTimeZone> for TantivyValue {}
