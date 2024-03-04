use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use deltalake::datafusion::arrow::datatypes::*;
use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::*;
use thiserror::Error;

use super::datatype::PgTypeMod;

const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

pub struct Microseconds(pub i64);

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

impl TryInto<Microseconds> for datum::Timestamp {
    type Error = TimestampError;

    fn try_into(self) -> Result<Microseconds, TimestampError> {
        let date = NaiveDate::from_ymd_opt(self.year(), self.month().into(), self.day().into())
            .ok_or(TimestampError::Date(self.year(), self.month(), self.day()))?;

        let time = NaiveTime::from_hms_milli_opt(
            self.hour().into(),
            self.minute().into(),
            self.second() as u32,
            self.microseconds(),
        )
        .ok_or(TimestampError::Time(
            self.hour(),
            self.minute(),
            self.second(),
        ))?;

        let microseconds = TimestampMicrosecondType::make_value(NaiveDateTime::new(date, time))
            .ok_or(TimestampError::DateTime())?;

        Ok(Microseconds(microseconds))
    }
}

impl TryInto<Option<pg_sys::Datum>> for Microseconds {
    type Error = TimestampError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, TimestampError> {
        let Microseconds(microseconds) = self;
        let datetime = NaiveDateTime::from_timestamp_micros(microseconds)
            .ok_or(TimestampError::Microseconds(microseconds))?;

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
}

#[derive(Error, Debug)]
pub enum TimestampError {
    #[error(transparent)]
    DateTimeConversionError(#[from] DateTimeConversionError),

    #[error("Time {0}:{1}::{2} is invalid")]
    Time(u8, u8, f64),

    #[error("Date {0}-{1}-{2} is invalid")]
    Date(i32, u8, u8),

    #[error("Failed to make datetime")]
    DateTime(),

    #[error("Failed to convert {0} microseconds to datetime")]
    Microseconds(i64),

    #[error("Type timestamp({0}) is supported. Supported types are timestamp(0), timestamp(3), timestamp(6), and timestamp.")]
    UnsupportedTypeMod(i32),

    #[error("Nanosecond TimeUnit not supported")]
    UnsupportedNanosecond(),
}
