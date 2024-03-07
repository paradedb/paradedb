use chrono::{Datelike, Duration, NaiveDate};
use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::*;
use thiserror::Error;

const EPOCH_YEAR: i32 = 1970;
const EPOCH_MONTH: u32 = 1;
const EPOCH_DAY: u32 = 1;

pub struct DayUnix(pub i32);

impl TryFrom<datum::Date> for DayUnix {
    type Error = DateError;

    fn try_from(date: datum::Date) -> Result<Self, Self::Error> {
        Ok(DayUnix(date.to_unix_epoch_days()))
    }
}

impl TryFrom<DayUnix> for datum::Date {
    type Error = DateError;

    fn try_from(day: DayUnix) -> Result<Self, Self::Error> {
        let DayUnix(days_since_epoch) = day;
        let epoch = NaiveDate::from_ymd_opt(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY)
            .ok_or(DateError::InvalidEpoch)?;
        let date = epoch + Duration::days(days_since_epoch.into());

        Ok(datum::Date::new(
            date.year(),
            date.month() as u8,
            date.day() as u8,
        )?)
    }
}

#[derive(Error, Debug)]
pub enum DateError {
    #[error(transparent)]
    DateTimeConversion(#[from] DateTimeConversionError),

    #[error("Failed to set epoch {}-{}-{}", EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY)]
    InvalidEpoch,
}
