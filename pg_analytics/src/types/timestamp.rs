use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use deltalake::datafusion::arrow::datatypes::*;
use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::*;
use thiserror::Error;

const MICROSECONDS_IN_SECOND: u32 = 1_000_000;
const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

use super::datatype::PgTypeMod;

pub struct MicrosecondUnix(pub i64);
pub struct MillisecondUnix(pub i64);
pub struct SecondUnix(pub i64);

#[derive(Copy, Clone)]
pub enum PgTimestampPrecision {
    Default = -1,
    Second = 0,
    Millisecond = 3,
    Microsecond = 6,
}

impl PgTimestampPrecision {
    pub fn value(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<PgTypeMod> for PgTimestampPrecision {
    type Error = TimestampError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        let PgTypeMod(typemod) = typemod;

        match typemod {
            -1 => Ok(PgTimestampPrecision::Default),
            0 => Ok(PgTimestampPrecision::Second),
            3 => Ok(PgTimestampPrecision::Millisecond),
            6 => Ok(PgTimestampPrecision::Microsecond),
            unsupported => Err(TimestampError::UnsupportedTypeMod(unsupported)),
        }
    }
}

impl TryFrom<PgTypeMod> for TimeUnit {
    type Error = TimestampError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        match PgTimestampPrecision::try_from(typemod)? {
            PgTimestampPrecision::Default => Ok(TimeUnit::Microsecond),
            PgTimestampPrecision::Second => Ok(TimeUnit::Second),
            PgTimestampPrecision::Millisecond => Ok(TimeUnit::Millisecond),
            PgTimestampPrecision::Microsecond => Ok(TimeUnit::Microsecond),
        }
    }
}

impl TryFrom<TimeUnit> for PgTypeMod {
    type Error = TimestampError;

    fn try_from(unit: TimeUnit) -> Result<Self, Self::Error> {
        match unit {
            TimeUnit::Second => Ok(PgTypeMod(PgTimestampPrecision::Second.value())),
            TimeUnit::Millisecond => Ok(PgTypeMod(PgTimestampPrecision::Millisecond.value())),
            TimeUnit::Microsecond => Ok(PgTypeMod(PgTimestampPrecision::Microsecond.value())),
            TimeUnit::Nanosecond => Err(TimestampError::UnsupportedNanosecond()),
        }
    }
}

impl TryFrom<datum::Timestamp> for MicrosecondUnix {
    type Error = TimestampError;

    fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
        let date = get_naive_date(&timestamp)?;
        let time = get_naive_time(&timestamp)?;
        let unix = TimestampMicrosecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(MicrosecondUnix(unix))
    }
}

impl TryFrom<datum::Timestamp> for MillisecondUnix {
    type Error = TimestampError;

    fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
        let date = get_naive_date(&timestamp)?;
        let time = get_naive_time(&timestamp)?;
        let unix = TimestampMillisecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(MillisecondUnix(unix))
    }
}

impl TryFrom<datum::Timestamp> for SecondUnix {
    type Error = TimestampError;

    fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
        let date = get_naive_date(&timestamp)?;
        let time = get_naive_time(&timestamp)?;
        let unix = TimestampSecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(SecondUnix(unix))
    }
}

impl TryFrom<MicrosecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_from(micros: MicrosecondUnix) -> Result<Self, Self::Error> {
        let MicrosecondUnix(unix) = micros;
        let datetime = NaiveDateTime::from_timestamp_micros(unix)
            .ok_or(TimestampError::MicrosecondsConversion(unix))?;

        to_timestamp(&datetime)
    }
}

impl TryFrom<MillisecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_from(millis: MillisecondUnix) -> Result<Self, Self::Error> {
        let MillisecondUnix(unix) = millis;
        let datetime = NaiveDateTime::from_timestamp_millis(unix)
            .ok_or(TimestampError::MillisecondsConversion(unix))?;

        to_timestamp(&datetime)
    }
}

impl TryFrom<SecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_from(seconds: SecondUnix) -> Result<Self, Self::Error> {
        let SecondUnix(unix) = seconds;
        let datetime = NaiveDateTime::from_timestamp_opt(unix, 0)
            .ok_or(TimestampError::SecondsConversion(unix))?;

        to_timestamp(&datetime)
    }
}

#[inline]
fn get_naive_date(timestamp: &datum::Timestamp) -> Result<NaiveDate, TimestampError> {
    NaiveDate::from_ymd_opt(
        timestamp.year(),
        timestamp.month().into(),
        timestamp.day().into(),
    )
    .ok_or(TimestampError::ParseDate(timestamp.to_iso_string()))
}

#[inline]
fn get_naive_time(timestamp: &datum::Timestamp) -> Result<NaiveTime, TimestampError> {
    NaiveTime::from_hms_micro_opt(
        timestamp.hour().into(),
        timestamp.minute().into(),
        timestamp.second() as u32,
        timestamp.microseconds() % MICROSECONDS_IN_SECOND,
    )
    .ok_or(TimestampError::ParseTime(timestamp.to_iso_string()))
}

#[inline]
fn to_timestamp(datetime: &NaiveDateTime) -> Result<datum::Timestamp, TimestampError> {
    Ok(datum::Timestamp::new(
        datetime.year(),
        datetime.month() as u8,
        datetime.day() as u8,
        datetime.hour() as u8,
        datetime.minute() as u8,
        (datetime.second() + datetime.nanosecond() / NANOSECONDS_IN_SECOND).into(),
    )?)
}

#[derive(Error, Debug)]
pub enum TimestampError {
    #[error(transparent)]
    DateTimeConversion(#[from] DateTimeConversionError),

    #[error("Failed to parse time from {0:?}")]
    ParseTime(String),

    #[error("Failed to parse date from {0:?}")]
    ParseDate(String),

    #[error("Failed to make datetime")]
    ParseDateTime(),

    #[error("Failed to convert {0} microseconds to datetime")]
    MicrosecondsConversion(i64),

    #[error("Failed to convert {0} milliseconds to datetime")]
    MillisecondsConversion(i64),

    #[error("Failed to convert {0} seconds to datetime")]
    SecondsConversion(i64),

    #[error("Only timestamp and timestamp(6), not timestamp({0}), are supported")]
    UnsupportedTypeMod(i32),

    #[error("Unexpected nanosecond TimeUnit")]
    UnsupportedNanosecond(),
}
