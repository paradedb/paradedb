// Copyright (c) 2023-2024 Retake, Inc.
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

use chrono::{NaiveDate, NaiveDateTime};
use thiserror::Error;

static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

// TODO: create a conversion trait

fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    let naive_dt = match ymd {
        Some(ymd) => NaiveDate::from_ymd_opt(ymd.0, ymd.1.into(), ymd.2.into()).unwrap(),
        None => NaiveDateTime::UNIX_EPOCH.date(),
    }
    .and_hms_micro_opt(
        hms_micro.0.into(),
        hms_micro.1.into(),
        hms_micro.2.into(),
        hms_micro.3 % MICROSECONDS_IN_SECOND,
    )
    .ok_or(DatetimeConversionError::FailedPostgresToTantivy)?
    .and_utc();

    Ok(tantivy::schema::OwnedValue::Date(
        tantivy::DateTime::from_timestamp_micros(naive_dt.timestamp_micros()),
    ))
}

pub fn pgrx_time_to_tantivy_value(
    value: pgrx::Time,
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn tantivy_value_to_pgrx_time(
    value: tantivy::DateTime,
) -> Result<pgrx::Time, DatetimeConversionError> {
    let prim_dt = value.into_primitive();
    let (h, m, s, micro) = prim_dt.as_hms_micro();
    pgrx::Time::new(
        h,
        m,
        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
    )
    .map_err(|_| DatetimeConversionError::FailedTantivyToPostgres)
}

pub fn pgrx_timetz_to_tantivy_value(
    value: pgrx::TimeWithTimeZone,
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn tantivy_value_to_pgrx_timetz(
    value: tantivy::DateTime,
) -> Result<pgrx::TimeWithTimeZone, DatetimeConversionError> {
    let prim_dt = value.into_primitive();
    let (h, m, s, micro) = prim_dt.as_hms_micro();
    pgrx::TimeWithTimeZone::with_timezone(
        h,
        m,
        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
        "UTC",
    )
    .map_err(|_| DatetimeConversionError::FailedTantivyToPostgres)
}

pub fn pgrx_date_to_tantivy_value(
    value: pgrx::Date,
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (0, 0, 0, 0),
    )
}

pub fn tantivy_value_to_pgrx_date(
    value: tantivy::DateTime,
) -> Result<pgrx::Date, DatetimeConversionError> {
    let prim_dt = value.into_primitive();
    pgrx::Date::new(prim_dt.year(), prim_dt.month().into(), prim_dt.day())
        .map_err(|_| DatetimeConversionError::FailedTantivyToPostgres)
}

pub fn pgrx_timestamp_to_tantivy_value(
    value: pgrx::Timestamp,
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub fn tantivy_value_to_pgrx_timestamp(
    value: tantivy::DateTime,
) -> Result<pgrx::Timestamp, DatetimeConversionError> {
    let prim_dt = value.into_primitive();
    let (h, m, s, micro) = prim_dt.as_hms_micro();
    pgrx::Timestamp::new(
        prim_dt.year(),
        prim_dt.month().into(),
        prim_dt.day(),
        h,
        m,
        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
    )
    .map_err(|_| DatetimeConversionError::FailedTantivyToPostgres)
}

pub fn pgrx_timestamptz_to_tantivy_value(
    value: pgrx::TimestampWithTimeZone,
) -> Result<tantivy::schema::OwnedValue, DatetimeConversionError> {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub fn tantivy_value_to_pgrx_timestamptz(
    value: tantivy::DateTime,
) -> Result<pgrx::TimestampWithTimeZone, DatetimeConversionError> {
    let prim_dt = value.into_primitive();
    let (h, m, s, micro) = prim_dt.as_hms_micro();
    pgrx::TimestampWithTimeZone::with_timezone(
        prim_dt.year(),
        prim_dt.month().into(),
        prim_dt.day(),
        h,
        m,
        s as f64 + ((micro as f64) / (MICROSECONDS_IN_SECOND as f64)),
        "UTC",
    )
    .map_err(|_| DatetimeConversionError::FailedTantivyToPostgres)
}

#[derive(Error, Debug)]
pub enum DatetimeConversionError {
    #[error("Could not convert postgres datetime to TantivyValue")]
    FailedPostgresToTantivy,

    #[error("Could not convert TantivyValue datetime to postgres")]
    FailedTantivyToPostgres,
}
