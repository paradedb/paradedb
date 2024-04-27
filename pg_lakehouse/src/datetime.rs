use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use pgrx::*;
use thiserror::Error;

const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

#[derive(Clone, Debug)]
pub struct Date(pub NaiveDate);

#[derive(Clone, Debug)]
pub struct DateTime(pub NaiveDateTime);

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
