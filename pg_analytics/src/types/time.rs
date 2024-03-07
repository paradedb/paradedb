use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::*;
use thiserror::Error;

#[derive(Copy, Clone, Debug)]
pub struct MicrosecondDay(pub i64);

const MICROSECONDS_IN_SECOND: i64 = 1_000_000;
const MICROSECONDS_IN_MINUTE: i64 = MICROSECONDS_IN_SECOND * 60;
const MICROSECONDS_IN_HOUR: i64 = MICROSECONDS_IN_MINUTE * 60;

impl TryFrom<datum::Time> for MicrosecondDay {
    type Error = TimeError;

    fn try_from(time: datum::Time) -> Result<Self, Self::Error> {
        let micros_elapsed = time.microseconds()
            + (time.minute() as u32) * (MICROSECONDS_IN_MINUTE as u32)
            + (time.hour() as u32) * (MICROSECONDS_IN_HOUR as u32);

        Ok(MicrosecondDay(micros_elapsed as i64))
    }
}

impl TryFrom<MicrosecondDay> for datum::Time {
    type Error = TimeError;

    fn try_from(micros: MicrosecondDay) -> Result<Self, Self::Error> {
        let MicrosecondDay(micros) = micros;

        let hours = micros / MICROSECONDS_IN_HOUR;
        let minutes = (micros % MICROSECONDS_IN_HOUR) / MICROSECONDS_IN_MINUTE;
        let seconds = (micros % MICROSECONDS_IN_MINUTE) / MICROSECONDS_IN_SECOND;
        let microseconds = micros % MICROSECONDS_IN_SECOND;
        let total_seconds = seconds as f64 + (microseconds as f64 / MICROSECONDS_IN_SECOND as f64);

        Ok(datum::Time::new(hours as u8, minutes as u8, total_seconds)?)
    }
}

#[derive(Error, Debug)]
pub enum TimeError {
    #[error(transparent)]
    DateTimeConversion(#[from] DateTimeConversionError),

    #[error("Unsupported time precision time({0})")]
    UnsupportedTypeMod(i32),
}
