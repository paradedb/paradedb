use chrono::{NaiveTime, TimeDelta, Timelike};
use deltalake::datafusion::arrow::datatypes::TimeUnit;
use pgrx::*;
use thiserror::Error;

use super::datatype::PgTypeMod;

const NANOSECONDS_IN_SECOND: i64 = 1_000_000_000;
const NANOSECONDS_IN_MINUTE: i64 = NANOSECONDS_IN_SECOND * 60;
const NANOSECONDS_IN_HOUR: i64 = NANOSECONDS_IN_MINUTE * 60;

#[derive(Copy, Clone, Debug)]
pub struct NanosecondDay(pub i64);

#[derive(Clone, Debug)]
pub struct TimePrecision(pub TimeUnit);

#[derive(Copy, Clone)]
pub enum PgTimePrecision {
    Default = -1,
    Microsecond = 6,
}

impl PgTimePrecision {
    pub fn value(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<PgTypeMod> for PgTimePrecision {
    type Error = TimeError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        let PgTypeMod(typemod) = typemod;

        match typemod {
            -1 => Ok(PgTimePrecision::Default),
            6 => Ok(PgTimePrecision::Microsecond),
            unsupported => Err(TimeError::UnsupportedTypeMod(unsupported)),
        }
    }
}

// Tech Debt: DataFusion defaults time fields with no specified precision to nanosecond,
// whereas Postgres defaults to microsecond. DataFusion errors when we try to compare
// Time64(Nanosecond) with Time64(Microsecond), so we just store microsecond precision
// times as nanosecond as a workaround
impl TryFrom<PgTypeMod> for TimePrecision {
    type Error = TimeError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        match PgTimePrecision::try_from(typemod)? {
            PgTimePrecision::Default => Ok(TimePrecision(TimeUnit::Nanosecond)),
            PgTimePrecision::Microsecond => Ok(TimePrecision(TimeUnit::Nanosecond)),
        }
    }
}

impl TryFrom<TimePrecision> for PgTypeMod {
    type Error = TimeError;

    fn try_from(unit: TimePrecision) -> Result<Self, Self::Error> {
        let TimePrecision(unit) = unit;

        match unit {
            TimeUnit::Nanosecond => Ok(PgTypeMod(PgTimePrecision::Microsecond.value())),
            unsupported => Err(TimeError::UnsupportedTimeUnit(unsupported)),
        }
    }
}

impl TryFrom<datum::Time> for NanosecondDay {
    type Error = TimeError;

    fn try_from(time: datum::Time) -> Result<Self, Self::Error> {
        let nanos_elapsed = (time.microseconds() as i64) * 1000
            + (time.minute() as i64) * NANOSECONDS_IN_MINUTE
            + (time.hour() as i64) * NANOSECONDS_IN_HOUR;

        Ok(NanosecondDay(nanos_elapsed))
    }
}

impl TryFrom<NanosecondDay> for datum::Time {
    type Error = TimeError;

    fn try_from(nanos: NanosecondDay) -> Result<Self, Self::Error> {
        let NanosecondDay(nanos) = nanos;

        let time_delta = TimeDelta::nanoseconds(nanos);
        let time = NaiveTime::from_hms_nano_opt(0, 0, 0, 0).ok_or(TimeError::MidnightNotFound)?
            + time_delta;
        let total_seconds =
            time.second() as f64 + time.nanosecond() as f64 / NANOSECONDS_IN_SECOND as f64;

        Ok(datum::Time::new(
            time.hour() as u8,
            time.minute() as u8,
            total_seconds,
        )?)
    }
}

#[derive(Error, Debug)]
pub enum TimeError {
    #[error(transparent)]
    DateTimeConversion(#[from] datum::datetime_support::DateTimeConversionError),

    #[error("Could not convert midnight to NaiveTime")]
    MidnightNotFound,

    #[error("Only time and time(6), not time({0}), are supported")]
    UnsupportedTypeMod(i32),

    #[error("Unexpected precision {0:?} for time")]
    UnsupportedTimeUnit(TimeUnit),
}
