use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Datelike, Timelike};
use pgrx::*;

const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

pub trait IntoTimestampMicrosecond {
    fn into_timestamp_microsecond(self) -> i64
}

impl IntoTimestampMicrosecond for datum::Timestamp {
    fn into_timestamp_microsecond(self) -> i64 {
        let date = NaiveDate::from_ymd_opt(
            self.year(),
            self.month() as u32,
            self.day() as u32,
        )?;
        let time = NaiveTime::from_hms_milli_opt(
            self.hour() as u32,
            self.minute() as u32,
            self.second() as u32,
            self.microseconds(),
        )?;

        TimestampMicrosecondType::make_value(NaiveDateTime::new(date, time))
    }
}

trait IntoTimstampDatum {
    fn into_timestamp_datum(self) -> Option<Datum>;
}

impl IntoTimestampDatum for i64 {
    fn into_timestamp_datum(self) -> Option<Datum> {
        match self {
            Some(microseconds) => {
                let datetime = NaiveDateTime::from_timestamp_micros(microseconds).unwrap();

                Some(datum::Timestamp::new(
                    datetime.year(),
                    datetime.month() as u8,
                    datetime.day() as u8,
                    datetime.hour() as u8,
                    datetime.minute() as u8,
                    (datetime.second() + datetime.nanosecond() / NANOSECONDS_IN_SECOND) as f64,
                ).unwrap().into_datum()?)
            },
            None => None
        }
    }
}