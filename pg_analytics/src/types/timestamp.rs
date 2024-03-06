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

impl TryInto<TimeUnit> for PgTypeMod {
    type Error = TimestampError;

    fn try_into(self) -> Result<TimeUnit, TimestampError> {
        match PgTimestampPrecision::try_from(self)? {
            PgTimestampPrecision::Default => Ok(TimeUnit::Microsecond),
            PgTimestampPrecision::Second => Ok(TimeUnit::Second),
            PgTimestampPrecision::Millisecond => Ok(TimeUnit::Millisecond),
            PgTimestampPrecision::Microsecond => Ok(TimeUnit::Microsecond),
        }
    }
}

impl TryInto<PgTypeMod> for TimeUnit {
    type Error = TimestampError;

    fn try_into(self) -> Result<PgTypeMod, TimestampError> {
        match self {
            TimeUnit::Second => Ok(PgTypeMod(PgTimestampPrecision::Second.value())),
            TimeUnit::Millisecond => Ok(PgTypeMod(PgTimestampPrecision::Millisecond.value())),
            TimeUnit::Microsecond => Ok(PgTypeMod(PgTimestampPrecision::Microsecond.value())),
            TimeUnit::Nanosecond => Err(TimestampError::UnsupportedNanosecond()),
        }
    }
}

impl TryInto<MicrosecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_into(self) -> Result<MicrosecondUnix, TimestampError> {
        let date = get_naive_date(&self)?;
        let time = get_naive_time(&self)?;
        let unix = TimestampMicrosecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(MicrosecondUnix(unix))
    }
}

impl TryInto<MillisecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_into(self) -> Result<MillisecondUnix, TimestampError> {
        let date = get_naive_date(&self)?;
        let time = get_naive_time(&self)?;
        let unix = TimestampMillisecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(MillisecondUnix(unix))
    }
}

impl TryInto<SecondUnix> for datum::Timestamp {
    type Error = TimestampError;

    fn try_into(self) -> Result<SecondUnix, TimestampError> {
        let date = get_naive_date(&self)?;
        let time = get_naive_time(&self)?;
        let unix = TimestampSecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::ParseDateTime())?;

        Ok(SecondUnix(unix))
    }
}

impl TryInto<Option<pg_sys::Datum>> for MicrosecondUnix {
    type Error = TimestampError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, TimestampError> {
        let MicrosecondUnix(unix) = self;
        let datetime = NaiveDateTime::from_timestamp_micros(unix)
            .ok_or(TimestampError::MicrosecondsConversion(unix))?;

        into_datum(&datetime)
    }
}

impl TryInto<Option<pg_sys::Datum>> for MillisecondUnix {
    type Error = TimestampError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, TimestampError> {
        let MillisecondUnix(unix) = self;
        let datetime = NaiveDateTime::from_timestamp_millis(unix)
            .ok_or(TimestampError::MillisecondsConversion(unix))?;

        into_datum(&datetime)
    }
}

impl TryInto<Option<pg_sys::Datum>> for SecondUnix {
    type Error = TimestampError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, TimestampError> {
        let SecondUnix(unix) = self;
        let datetime = NaiveDateTime::from_timestamp_opt(unix, 0)
            .ok_or(TimestampError::SecondsConversion(unix))?;

        into_datum(&datetime)
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
fn into_datum(datetime: &NaiveDateTime) -> Result<Option<pg_sys::Datum>, TimestampError> {
    Ok(datum::Timestamp::new(
        datetime.year(),
        datetime.month() as u8,
        datetime.day() as u8,
        datetime.hour() as u8,
        datetime.minute() as u8,
        (datetime.second() + datetime.nanosecond() / NANOSECONDS_IN_SECOND).into(),
    )?
    .into_datum())
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

    #[error("Type timestamp({0}) is supported. Supported types are timestamp(0), timestamp(3), timestamp(6), and timestamp.")]
    UnsupportedTypeMod(i32),

    #[error("Unexpected nanosecond TimeUnit")]
    UnsupportedNanosecond(),
}
