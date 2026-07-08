// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use pgrx::datum::datetime_support::DateTimeConversionError;
use pgrx::datum::ToIsoString;
use serde::{Deserialize, Serialize};

pub static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

/// The Unix epoch is midnight 1970-01-01. The postgres epoch is midnight 2000-01-01.
/// This is the difference between them in microseconds.
pub static PG_EPOCH_DIFF_FROM_UNIX_EPOCH_MICROS: i64 = 946_684_800 * MICROSECONDS_IN_SECOND as i64;

#[cfg(any(test, feature = "pg_test"))]
pub fn pg_micros_to_unix_micros(pg_micros: i64) -> i64 {
    pg_micros
        .checked_add(PG_EPOCH_DIFF_FROM_UNIX_EPOCH_MICROS)
        .unwrap()
}

pub fn unix_micros_to_pg_micros(unix_micros: i64) -> i64 {
    unix_micros
        .checked_sub(PG_EPOCH_DIFF_FROM_UNIX_EPOCH_MICROS)
        .unwrap()
}

pub fn unix_millis_to_pg_micros(unix_millis: i64) -> i64 {
    let unix_micros = unix_millis.checked_mul(1_000).unwrap();
    unix_micros_to_pg_micros(unix_micros)
}

/// The minimum nanoseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without underflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MIN_SAFE_TANTIVY_NANOS: i64 =
    (i64::MIN / 1_000_000_000 / 86_400) * 86_400 * 1_000_000_000;

/// The maximum nanoseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without overflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MAX_SAFE_TANTIVY_NANOS: i64 =
    (i64::MAX / 1_000_000_000 / 86_400) * 86_400 * 1_000_000_000;

/// The minimum microseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without underflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MIN_SAFE_TANTIVY_UNIX_MICROS: i64 =
    (i64::MIN / 1_000_000_000 / 86_400) * 86_400 * 1_000_000;

/// The maximum microseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without overflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MAX_SAFE_TANTIVY_UNIX_MICROS: i64 =
    (i64::MAX / 1_000_000_000 / 86_400) * 86_400 * 1_000_000;

/// The minimum value storable by pgrx::datum::Timestamp. This has been copied here, for use in
/// tests, from pgrx source, which does not export it.
#[allow(dead_code)]
pub const MIN_PG_MICROS: i64 = -211_813_488_000_000_000;

/// The maximum value storable by pgrx::datum::Timestamp. This has been copied here, for use in
/// tests, from pgrx source, which does not export it.
#[allow(dead_code)]
pub const MAX_PG_MICROS: i64 = 9_223_371_331_200_000_000 - 1;

const SECOND_MICROS: i64 = 1_000_000;
const MINUTE_MICROS: i64 = 60 * SECOND_MICROS;
const HOUR_MICROS: i64 = 60 * MINUTE_MICROS;
const ONE_DAY_MICROS: i64 = 24 * HOUR_MICROS;

/// A wrapper type for working with postgres time values. Holds a postgres timestamp, which is
/// really just a wrapper around an i64 representing microseconds from the PG epoch.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct PostgresDateTime(pgrx::datum::Timestamp);

// Compare/hash/order via the inner i64 directly rather than `#[derive]`-ing. Deriving would route
// through `<pgrx::datum::Timestamp as PartialEq/Hash/PartialOrd>`, which pgrx implements (via
// `impl_wrappers!`) by calling PG backend functions through `pg_guard_ffi_boundary`. That wrapper
// references PG globals (`CurrentMemoryContext`, `PG_exception_stack`, `error_context_stack`) and
// `timestamp_eq/cmp/hash` — none of which are resolvable at test-binary link time, breaking CI
// against a system Postgres install.
impl PartialEq for PostgresDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.0.into_inner() == other.0.into_inner()
    }
}
impl Eq for PostgresDateTime {}
impl Hash for PostgresDateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.into_inner().hash(state);
    }
}
impl PartialOrd for PostgresDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.into_inner().partial_cmp(&other.0.into_inner())
    }
}

impl PostgresDateTime {
    pub fn into_inner(self) -> i64 {
        self.0.into_inner()
    }

    pub fn try_from_raw(raw: i64) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::Timestamp::try_from(raw)
            .map_err(|_| DateTimeConversionError::OutOfRange)?;
        Ok(Self(ts))
    }

    pub fn try_from_unix_nanos(unix_nanos: i64) -> Result<Self, DateTimeConversionError> {
        assert_eq!(unix_nanos % 1_000, 0, "We should never see a timestamp with greater than microsecond precision because postgres only supports microsecond precision");
        let unix_micros = unix_nanos / 1_000;
        Self::try_from_raw(unix_micros_to_pg_micros(unix_micros))
    }

    pub fn try_from_unix_micros(unix_micros: i64) -> Result<Self, DateTimeConversionError> {
        Self::try_from_raw(unix_micros_to_pg_micros(unix_micros))
    }

    pub fn try_from_timestamp_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::Timestamp::from_str(s)?;
        Ok(Self::from(ts))
    }

    pub fn try_from_timestamptz_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::TimestampWithTimeZone::from_str(s)?;
        Ok(Self::from(ts))
    }

    pub fn try_from_date_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let date = pgrx::datum::Date::from_str(s)?;
        Ok(Self::from(date))
    }

    pub fn try_from_time_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let time = pgrx::datum::Time::from_str(s)?;
        Ok(Self::from(time))
    }

    pub fn try_from_timetz_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let time = pgrx::datum::TimeWithTimeZone::from_str(s)?;
        Ok(Self::from(time))
    }

    pub fn add_days(&self, days: i64) -> Result<Self, DateTimeConversionError> {
        let plus_days_micros = ONE_DAY_MICROS
            .checked_mul(days)
            .ok_or(DateTimeConversionError::FieldOverflow)?;
        let new_micros = self
            .into_inner()
            .checked_add(plus_days_micros)
            .ok_or(DateTimeConversionError::FieldOverflow)?;
        Self::try_from_raw(new_micros)
    }
}
impl Display for PostgresDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        f.write_str(&s)
    }
}
impl TryFrom<String> for PostgresDateTime {
    type Error = DateTimeConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}
impl TryFrom<&str> for PostgresDateTime {
    type Error = DateTimeConversionError;

    /// Tantivy uses rfc3339 as it's supported date parsing method, so we'll replicate that here,
    /// as this is largely used for user-supplied datetime strings
    /// If you need something more lenient, use one of:
    /// - `PostgresDateTime::try_from_date_str`
    /// - `PostgresDateTime::try_from_timestamp_str`
    /// - `PostgresDateTime::try_from_timestamptz_str`
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if can_be_rfc3339_date_time(value) {
            // From the chrono docs about DateTime::from_str:
            // Accepts a relaxed form of RFC3339. A space or a ‘T’ are accepted as the separator
            // between the date and time parts. Additional spaces are allowed between each
            // component.
            if let Ok(dt) = chrono::DateTime::<chrono::FixedOffset>::from_str(value) {
                return Self::try_from(dt.to_utc());
            }
        }
        Err(DateTimeConversionError::InvalidFormat)
    }
}
/// Cheap way to skip full parsing of things that can't be valid rfc3339 dates
fn can_be_rfc3339_date_time(text: &str) -> bool {
    if let Some(&first_byte) = text.as_bytes().first() {
        if first_byte.is_ascii_digit() {
            return true;
        }
    }

    false
}

impl From<PostgresDateTime> for String {
    fn from(value: PostgresDateTime) -> Self {
        // append Z to make the output rfc3339-compliant
        format!("{}Z", value.0.to_iso_string())
    }
}
impl From<pgrx::datum::Date> for PostgresDateTime {
    fn from(value: pgrx::datum::Date) -> Self {
        let midnight_micros = (value.to_pg_epoch_days() as i64)
            .checked_mul(ONE_DAY_MICROS)
            .expect("days to micros should never overflow");
        Self::try_from_raw(midnight_micros)
            .expect("Date->Timestamp conversion should always be valid")
    }
}
impl From<PostgresDateTime> for pgrx::datum::Date {
    fn from(value: PostgresDateTime) -> Self {
        let pg_days: i32 = (value.into_inner() / ONE_DAY_MICROS)
            .try_into()
            .expect("This should always fit");
        unsafe { Self::from_pg_epoch_days(pg_days) }
    }
}
impl From<pgrx::datum::Time> for PostgresDateTime {
    fn from(value: pgrx::datum::Time) -> Self {
        Self::try_from_raw(value.into_inner())
            .expect("time micros -> Timestamp conversion should always work")
    }
}
impl From<PostgresDateTime> for pgrx::datum::Time {
    fn from(value: PostgresDateTime) -> Self {
        Self::modular_from_raw(value.into_inner())
    }
}
impl From<pgrx::datum::TimeWithTimeZone> for PostgresDateTime {
    fn from(value: pgrx::datum::TimeWithTimeZone) -> Self {
        Self::from(value.to_utc())
    }
}
impl From<PostgresDateTime> for pgrx::datum::TimeWithTimeZone {
    fn from(value: PostgresDateTime) -> Self {
        Self::from(pgrx::datum::Time::from(value))
    }
}
impl From<pgrx::datum::Timestamp> for PostgresDateTime {
    fn from(val: pgrx::datum::Timestamp) -> Self {
        Self(val)
    }
}
impl From<pgrx::datum::TimestampWithTimeZone> for PostgresDateTime {
    fn from(val: pgrx::datum::TimestampWithTimeZone) -> Self {
        // Postgres's TimestampWithTimeZone is just Timestamp with different logic for handling it when
        // returning it to the user. The internal representation is the same i64 microseconds from
        // the PG epoch that Timestamp, uses, so we are safe to just convert it here
        Self(val.to_utc())
    }
}
impl From<PostgresDateTime> for pgrx::datum::Timestamp {
    fn from(value: PostgresDateTime) -> Self {
        value.0
    }
}
impl From<PostgresDateTime> for pgrx::datum::TimestampWithTimeZone {
    fn from(value: PostgresDateTime) -> Self {
        // Postgres's TimestampWithTimeZone is just Timestamp with different logic for handling it when
        // returning it to the user. The internal representation is the same i64 microseconds from
        // the PG epoch that Timestamp, uses, so we are safe to just convert it here
        pgrx::datum::TimestampWithTimeZone::try_from(value.into_inner()).expect(
            "PostgresDateTime->pgrx::datum::TimestampWithTimeZone conversion should always work",
        )
    }
}
impl TryFrom<tantivy::DateTime> for PostgresDateTime {
    type Error = DateTimeConversionError;

    fn try_from(val: tantivy::DateTime) -> Result<Self, DateTimeConversionError> {
        let unix_micros = val.into_timestamp_micros();
        let pg_micros = unix_micros
            .checked_sub(PG_EPOCH_DIFF_FROM_UNIX_EPOCH_MICROS)
            .ok_or(DateTimeConversionError::OutOfRange)?;
        Self::try_from_raw(pg_micros)
    }
}
impl TryFrom<PostgresDateTime> for tantivy::DateTime {
    type Error = DateTimeConversionError;

    fn try_from(val: PostgresDateTime) -> Result<Self, DateTimeConversionError> {
        let unix_micros = val
            .into_inner()
            .checked_add(PG_EPOCH_DIFF_FROM_UNIX_EPOCH_MICROS)
            .ok_or(DateTimeConversionError::OutOfRange)?;
        if !(MIN_SAFE_TANTIVY_UNIX_MICROS..=MAX_SAFE_TANTIVY_UNIX_MICROS).contains(&unix_micros) {
            return Err(DateTimeConversionError::OutOfRange);
        }
        Ok(tantivy::DateTime::from_timestamp_micros(unix_micros))
    }
}
impl TryFrom<chrono::DateTime<chrono::Utc>> for PostgresDateTime {
    type Error = DateTimeConversionError;

    fn try_from(value: chrono::DateTime<chrono::Utc>) -> Result<Self, Self::Error> {
        Self::try_from_unix_micros(value.timestamp_micros())
    }
}

impl From<PostgresDateTime> for i64 {
    fn from(value: PostgresDateTime) -> Self {
        value.into_inner()
    }
}
impl TryFrom<i64> for PostgresDateTime {
    type Error = DateTimeConversionError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        PostgresDateTime::try_from_raw(value)
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::datum::{Timestamp, TimestampWithTimeZone};
    use pgrx::pg_test;
    use proptest::*;

    // 2024-01-01 00:00:00 UTC
    const UNIX_MICROS_2024: i64 = 1_704_067_200_000_000;
    const PG_MICROS_2024: i64 = 757_382_400_000_000;

    #[test]
    fn safe_tantivy_micros_are_entirely_in_bounds_of_safe_pg_micros() {
        let pg_safe_range = MIN_PG_MICROS..=MAX_PG_MICROS;
        assert!(pg_safe_range.contains(&unix_micros_to_pg_micros(MIN_SAFE_TANTIVY_UNIX_MICROS)));
        assert!(pg_safe_range.contains(&unix_micros_to_pg_micros(MAX_SAFE_TANTIVY_UNIX_MICROS)));
    }

    #[pg_test]
    fn tantivy_datetime_to_timestamp() {
        let tantivy_dt = tantivy::DateTime::from_timestamp_micros(UNIX_MICROS_2024);
        let pg_dt = PostgresDateTime::try_from(tantivy_dt).unwrap();
        let ts: Timestamp = pg_dt.into();
        assert_eq!(ts.into_inner(), PG_MICROS_2024);
    }

    #[pg_test]
    fn tantivy_datetime_to_postgres_datetime_bounds() {
        proptest!(|(unix_micros in MIN_SAFE_TANTIVY_UNIX_MICROS..=MAX_SAFE_TANTIVY_UNIX_MICROS)| {
            let tantivy_dt = tantivy::DateTime::from_timestamp_micros(unix_micros);
            let pg_dt = PostgresDateTime::try_from(tantivy_dt).unwrap();
            assert_eq!(pg_micros_to_unix_micros(pg_dt.into_inner()), unix_micros);
        });
    }

    #[pg_test]
    fn postgres_datetime_to_tantivy_datetime_bounds() {
        // in bounds
        proptest!(|(unix_micros in MIN_SAFE_TANTIVY_UNIX_MICROS..=MAX_SAFE_TANTIVY_UNIX_MICROS)| {
            let pg_dt = PostgresDateTime::try_from_raw(unix_micros_to_pg_micros(unix_micros)).unwrap();
            let tantivy_dt_res: Result<tantivy::DateTime, _> = pg_dt.try_into();
            assert_eq!(tantivy_dt_res.unwrap().into_timestamp_micros(), unix_micros);
        });

        // below
        let pg_dt = PostgresDateTime::try_from_raw(unix_micros_to_pg_micros(
            MIN_SAFE_TANTIVY_UNIX_MICROS - 1,
        ))
        .unwrap();
        let tantivy_dt_res: Result<tantivy::DateTime, _> = pg_dt.try_into();
        assert!(tantivy_dt_res.is_err());

        // above
        let pg_dt = PostgresDateTime::try_from_raw(unix_micros_to_pg_micros(
            MAX_SAFE_TANTIVY_UNIX_MICROS + 1,
        ))
        .unwrap();
        let tantivy_dt_res: Result<tantivy::DateTime, _> = pg_dt.try_into();
        assert!(tantivy_dt_res.is_err());
    }

    #[pg_test]
    fn tantivy_datetime_to_timestamptz() {
        let tantivy_dt = tantivy::DateTime::from_timestamp_micros(UNIX_MICROS_2024);
        let pg_dt = PostgresDateTime::try_from(tantivy_dt).unwrap();
        let ts: TimestampWithTimeZone = pg_dt.into();
        assert_eq!(ts.into_inner(), PG_MICROS_2024);
    }

    #[pg_test]
    fn timestamp_to_tantivy_datetime() {
        let ts = Timestamp::try_from(PG_MICROS_2024).unwrap();
        let pg_dt = PostgresDateTime::from(ts);
        let tantivy_dt = tantivy::DateTime::try_from(pg_dt).unwrap();
        assert_eq!(tantivy_dt.into_timestamp_micros(), UNIX_MICROS_2024);
    }

    #[pg_test]
    fn timestamptz_to_tantivy_datetime() {
        let ts = TimestampWithTimeZone::try_from(PG_MICROS_2024).unwrap();
        let pg_dt = PostgresDateTime::from(ts);
        let tantivy_dt = tantivy::DateTime::try_from(pg_dt).unwrap();
        assert_eq!(tantivy_dt.into_timestamp_micros(), UNIX_MICROS_2024);

        // An equivalent instant parsed from a non-zero-offset string should produce the same UTC tantivy DateTime.
        let ts_with_offset = TimestampWithTimeZone::from_str("2024-01-01 05:00:00+05:00").unwrap();
        let pg_dt = PostgresDateTime::from(ts_with_offset);
        let tantivy_dt = tantivy::DateTime::try_from(pg_dt).unwrap();
        assert_eq!(tantivy_dt.into_timestamp_micros(), UNIX_MICROS_2024);
    }

    #[pg_test]
    fn timestamp_round_trip_through_tantivy_datetime() {
        let original = Timestamp::try_from(PG_MICROS_2024).unwrap();
        let tantivy_dt = tantivy::DateTime::try_from(PostgresDateTime::from(original)).unwrap();
        let round_tripped: Timestamp = PostgresDateTime::try_from(tantivy_dt).unwrap().into();
        assert_eq!(original.into_inner(), round_tripped.into_inner());
    }

    #[pg_test]
    fn timestamptz_round_trip_through_tantivy_datetime() {
        let original = TimestampWithTimeZone::try_from(PG_MICROS_2024).unwrap();
        let tantivy_dt = tantivy::DateTime::try_from(PostgresDateTime::from(original)).unwrap();
        let round_tripped: TimestampWithTimeZone =
            PostgresDateTime::try_from(tantivy_dt).unwrap().into();
        assert_eq!(original.into_inner(), round_tripped.into_inner());

        // A value parsed from a non-zero-offset string should round-trip to the same UTC instant.
        let original_with_offset =
            TimestampWithTimeZone::from_str("2024-01-01 05:00:00+05:00").unwrap();
        let tantivy_dt =
            tantivy::DateTime::try_from(PostgresDateTime::from(original_with_offset)).unwrap();
        let round_tripped: TimestampWithTimeZone =
            PostgresDateTime::try_from(tantivy_dt).unwrap().into();
        assert_eq!(
            original_with_offset.into_inner(),
            round_tripped.into_inner()
        );
        // verify our assertion that the inner value is in UTC
        assert_eq!(original_with_offset.into_inner(), PG_MICROS_2024);
    }

    #[pg_test]
    fn tantivy_datetime_roundtrip_through_postgres_datetime() {
        let tantivy_dt = tantivy::DateTime::from_timestamp_micros(UNIX_MICROS_2024);
        let pg_dt = PostgresDateTime::try_from(tantivy_dt).unwrap();
        let round_tripped: tantivy::DateTime = pg_dt.try_into().unwrap();
        assert_eq!(
            tantivy_dt.into_timestamp_micros(),
            round_tripped.into_timestamp_micros()
        );
    }
}
