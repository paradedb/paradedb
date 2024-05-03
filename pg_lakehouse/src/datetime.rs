use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use pgrx::*;
use thiserror::Error;

const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

#[derive(Clone, Debug)]
pub struct Date(pub NaiveDate);

#[derive(Clone, Debug)]
pub struct DateTime(pub NaiveDateTime);

#[derive(Copy, Clone, Debug)]
pub struct PgTypeMod(pub i32);

#[derive(Copy, Clone)]
pub enum PgTimestampPrecision {
    Default = -1,
    Second = 0,
    Millisecond = 3,
    Microsecond = 6,
}

impl TryFrom<PgTypeMod> for PgTimestampPrecision {
    type Error = DatetimeError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        let PgTypeMod(typemod) = typemod;

        match typemod {
            -1 => Ok(PgTimestampPrecision::Default),
            1 => Ok(PgTimestampPrecision::Second),
            3 => Ok(PgTimestampPrecision::Millisecond),
            6 => Ok(PgTimestampPrecision::Microsecond),
            unsupported => Err(DatetimeError::UnsupportedTimestampPrecision(unsupported)),
        }
    }
}

impl TryFrom<DateTime> for datum::Timestamp {
    type Error = datum::datetime_support::DateTimeConversionError;

    fn try_from(datetime: DateTime) -> Result<Self, Self::Error> {
        let DateTime(datetime) = datetime;

        datum::Timestamp::new(
            datetime.year(),
            datetime.month() as u8,
            datetime.day() as u8,
            datetime.hour() as u8,
            datetime.minute() as u8,
            (datetime.second() + datetime.nanosecond() / NANOSECONDS_IN_SECOND).into(),
        )
    }
}

impl TryFrom<Date> for datum::Date {
    type Error = datum::datetime_support::DateTimeConversionError;

    fn try_from(date: Date) -> Result<Self, Self::Error> {
        let Date(date) = date;
        datum::Date::new(date.year(), date.month() as u8, date.day() as u8)
    }
}

#[derive(Error, Debug)]
pub enum DatetimeError {
    #[error("Precision ({0}) is not supported for TIMESTAMP. Supported precisions are 1 for second, 3 for millisecond, and 6 for microsecond")]
    UnsupportedTimestampPrecision(i32),
}
