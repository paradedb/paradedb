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

static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> tantivy::schema::OwnedValue {
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
    .unwrap()
    .and_utc();

    tantivy::schema::OwnedValue::Date(tantivy::DateTime::from_timestamp_micros(
        naive_dt.timestamp_micros(),
    ))
}

pub fn pgrx_time_to_tantivy_value(value: pgrx::Time) -> tantivy::schema::OwnedValue {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_timetz_to_tantivy_value(value: pgrx::TimeWithTimeZone) -> tantivy::schema::OwnedValue {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_date_to_tantivy_value(value: pgrx::Date) -> tantivy::schema::OwnedValue {
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (0, 0, 0, 0),
    )
}

pub fn pgrx_timestamp_to_tantivy_value(value: pgrx::Timestamp) -> tantivy::schema::OwnedValue {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub fn pgrx_timestamptz_to_tantivy_value(
    value: pgrx::TimestampWithTimeZone,
) -> tantivy::schema::OwnedValue {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}
