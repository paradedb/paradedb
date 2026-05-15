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

use std::str::FromStr;

use chrono::{DateTime, NaiveDate};
use pgrx::datum::datetime_support::DateTimeConversionError;

pub static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

/// The Unix epoch is midnight 1970-01-01. The postgres epoch is midnight 2000-01-01.
/// This is the difference between them in microseconds.
pub static PG_EPOCH_DIFF_FROM_UNIX_EPHOCH_MICROS: i64 = 946_684_800 * MICROSECONDS_IN_SECOND as i64;

pub fn pg_micros_to_unix_micros(pg_micros: i64) -> i64 {
    pg_micros
        .checked_add(PG_EPOCH_DIFF_FROM_UNIX_EPHOCH_MICROS)
        .unwrap()
}

#[cfg(test)]
pub fn unix_micros_to_pg_micros(unix_micros: i64) -> i64 {
    unix_micros
        .checked_sub(PG_EPOCH_DIFF_FROM_UNIX_EPHOCH_MICROS)
        .unwrap()
}

/// The minimum nanoseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without underflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MIN_SAFE_TANTIVY_MICROS: i64 = (i64::MIN / 1_000_000_000 / 86_400) * 86_400 * 1_000_000;

/// The maximum nanoseconds from 1970-01-01 00:00:00 UTC that can be safely
/// converted between Postgres types and Tantivy without overflowing i64 when floored to the
/// day.
#[allow(dead_code)]
pub const MAX_SAFE_TANTIVY_MICROS: i64 = (i64::MAX / 1_000_000_000 / 86_400) * 86_400 * 1_000_000;

#[inline]
pub fn micros_to_tantivy_datetime(
    micros: i64,
) -> Result<tantivy::DateTime, DateTimeConversionError> {
    let nanos = micros
        .checked_mul(1_000)
        .ok_or(DateTimeConversionError::OutOfRange)?;
    Ok(tantivy::DateTime::from_timestamp_nanos(nanos))
}

pub fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> Result<tantivy::schema::OwnedValue, DateTimeConversionError> {
    let naive_dt = match ymd {
        Some(ymd) => NaiveDate::from_ymd_opt(ymd.0, ymd.1.into(), ymd.2.into())
            .expect("ymd should be valid for NaiveDate::from_ymd_opt"),
        None => DateTime::UNIX_EPOCH.date_naive(),
    }
    .and_hms_micro_opt(
        hms_micro.0.into(),
        hms_micro.1.into(),
        hms_micro.2.into(),
        hms_micro.3 % MICROSECONDS_IN_SECOND,
    )
    .ok_or(DateTimeConversionError::OutOfRange)?
    .and_utc();

    Ok(tantivy::schema::OwnedValue::Date(
        micros_to_tantivy_datetime(naive_dt.timestamp_micros())?,
    ))
}

/// A wrapper type for working with postgres time values. Holds a postgres timestamp
#[derive(Clone, Copy, Debug)]
pub struct PostgresDateTime(pgrx::datum::Timestamp);
impl PostgresDateTime {
    pub fn into_inner(self) -> i64 {
        self.0.into_inner()
    }

    pub fn try_from_raw(raw: i64) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::Timestamp::try_from(raw)
            .map_err(|_| DateTimeConversionError::OutOfRange)?;
        Ok(Self(ts))
    }

    pub fn try_from_timestamp_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::Timestamp::from_str(s)?;
        Ok(Self::from(ts))
    }

    pub fn try_from_timestamptz_str(s: &str) -> Result<Self, DateTimeConversionError> {
        let ts = pgrx::datum::TimestampWithTimeZone::from_str(s)?;
        Ok(Self::from(ts))
    }
}
impl From<pgrx::datum::Timestamp> for PostgresDateTime {
    fn from(val: pgrx::datum::Timestamp) -> Self {
        Self(val)
    }
}
impl From<pgrx::datum::TimestampWithTimeZone> for PostgresDateTime {
    fn from(val: pgrx::datum::TimestampWithTimeZone) -> Self {
        // TODO: I'm not sure this shouldn't just take the inner value
        // I think in effect to_utc does nothing to the inner value, but I need to verify
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
            .checked_sub(PG_EPOCH_DIFF_FROM_UNIX_EPHOCH_MICROS)
            .ok_or(DateTimeConversionError::OutOfRange)?;
        Self::try_from_raw(pg_micros)
    }
}
impl TryFrom<PostgresDateTime> for tantivy::DateTime {
    type Error = DateTimeConversionError;

    fn try_from(val: PostgresDateTime) -> Result<Self, DateTimeConversionError> {
        let unix_micros = val
            .into_inner()
            .checked_add(PG_EPOCH_DIFF_FROM_UNIX_EPHOCH_MICROS)
            .ok_or(DateTimeConversionError::OutOfRange)?;
        // TODO: Assert bounds about min and max safe tantivy micros
        Ok(tantivy::DateTime::from_timestamp_micros(unix_micros))
    }
}
