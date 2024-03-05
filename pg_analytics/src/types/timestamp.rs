use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use deltalake::datafusion::arrow::datatypes::*;
use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::*;
use thiserror::Error;

use super::datatype::PgTypeMod;

const MICROSECONDS_IN_SECOND: u32 = 1_000_000;
const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

pub struct MicrosecondUnix(pub i64);
pub struct MillisecondUnix(pub i64);
pub struct SecondUnix(pub i64);

impl TryInto<TimeUnit> for PgTypeMod {
    type Error = TimestampError;

    fn try_into(self) -> Result<TimeUnit, TimestampError> {
        let PgTypeMod(typemod) = self;

        match typemod {
            -1 | 6 => Ok(TimeUnit::Microsecond),
            0 => Ok(TimeUnit::Second),
            3 => Ok(TimeUnit::Millisecond),
            unsupported => Err(TimestampError::UnsupportedTypeMod(unsupported)),
        }
    }
}

impl TryInto<PgTypeMod> for TimeUnit {
    type Error = TimestampError;

    fn try_into(self) -> Result<PgTypeMod, TimestampError> {
        match self {
            TimeUnit::Second => Ok(PgTypeMod(0)),
            TimeUnit::Millisecond => Ok(PgTypeMod(3)),
            TimeUnit::Microsecond => Ok(PgTypeMod(6)),
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

pub fn into_unix(
    timestamp: Option<datum::Timestamp>,
    typemod: i32,
) -> Result<Option<i64>, TimestampError> {
    if let Some(timestamp) = timestamp {
        match typemod {
            0 => {
                let SecondUnix(unix) = timestamp.try_into()?;
                Ok(Some(unix))
            }
            3 => {
                let MillisecondUnix(unix) = timestamp.try_into()?;
                Ok(Some(unix))
            }
            -1 | 6 => {
                let MicrosecondUnix(unix) = timestamp.try_into()?;
                Ok(Some(unix))
            }
            unsupported => Err(TimestampError::UnsupportedTypeMod(unsupported)),
        }
    } else {
        Ok(None)
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
