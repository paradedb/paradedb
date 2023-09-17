use pgrx::{Date, Time, TimeWithTimeZone, Timestamp, TimestampWithTimeZone, ToIsoString};
use serde::{Serialize, Serializer};

#[derive(Debug)]
#[repr(transparent)]
pub struct ParadeTimestamp(pub String);

#[derive(Debug)]
#[repr(transparent)]
pub struct ParadeTime(pub String);

#[derive(Debug)]
#[repr(transparent)]
pub struct ParadeTimeWithTimeZone(pub String);

#[derive(Debug)]
#[repr(transparent)]
pub struct ParadeDate(pub String);

#[derive(Debug)]
#[repr(transparent)]
pub struct ParadeTimestampWithTimeZone(pub String);

impl From<Timestamp> for ParadeTimestamp {
    fn from(ts: Timestamp) -> Self {
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        return ParadeTimestamp(ts.to_iso_string_with_timezone("UTC").unwrap() + "-00");

        #[cfg(not(any(feature = "pg13", feature = "pg14", feature = "pg15")))]
        return ParadeTimestamp(ts.to_iso_string() + "-00");
    }
}

impl From<TimestampWithTimeZone> for ParadeTimestampWithTimeZone {
    fn from(tsz: TimestampWithTimeZone) -> Self {
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        return ParadeTimestampWithTimeZone(tsz.to_iso_string_with_timezone("UTC").unwrap());

        #[cfg(not(any(feature = "pg13", feature = "pg14", feature = "pg15")))]
        return ParadeTimestampWithTimeZone(tsz.to_utc().to_iso_string() + "+00:00");
    }
}

impl From<Time> for ParadeTime {
    fn from(t: Time) -> Self {
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15"))]
        return ParadeTime(t.to_iso_string_with_timezone("UTC").unwrap());

        #[cfg(not(any(feature = "pg13", feature = "pg14", feature = "pg15")))]
        return ParadeTime(t.to_iso_string());
    }
}

impl From<TimeWithTimeZone> for ParadeTimeWithTimeZone {
    fn from(tz: TimeWithTimeZone) -> Self {
        let seconds = tz.second();
        let second_left = seconds as u64;
        let second_right = seconds.to_string();
        let mut parts = second_right.split('.');
        let _ = parts.next();
        let right = parts.next().unwrap_or("0");
        let right = &right[0..6.min(right.len())];
        let s = format!(
            "{:02}:{:02}:{:02}{}{}",
            tz.hour(),
            tz.minute(),
            second_left,
            if right.parse::<u64>().unwrap() > 0 {
                format!(".{:}", right)
            } else {
                "".to_string()
            },
            if tz.timezone_offset() == 0 {
                "Z".to_string()
            } else {
                let hour = tz.timezone_hour();
                let neg = hour < 0;
                let hour = hour.abs();
                (if neg { "-" } else { "" }).to_string()
                    + &format!("{:02}", hour)
                    + &format!("{:02}", tz.timezone_minute())
            }
        );
        ParadeTimeWithTimeZone(s)
    }
}

impl From<Date> for ParadeDate {
    fn from(t: Date) -> Self {
        ParadeDate(t.to_iso_string())
    }
}

macro_rules! serialize {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }
    };
}

serialize!(ParadeTimestamp);
serialize!(ParadeTime);
serialize!(ParadeTimeWithTimeZone);
serialize!(ParadeDate);
serialize!(ParadeTimestampWithTimeZone);
