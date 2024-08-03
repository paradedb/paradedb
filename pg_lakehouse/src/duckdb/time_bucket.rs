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
use pgrx::iter::TableIterator;
use pgrx::*;
use std::fmt::{Display, Formatter};

pub enum TimeBucketInput {
    Date(Date),
    Timestamp(Timestamp),
}

pub enum TimeBucketOffset {
    Interval(Interval),
    Date(Date),
}

impl Display for TimeBucketInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeBucketInput::Date(input) => {
                write!(f, "{}::DATE", input.to_string())
            }
            TimeBucketInput::Timestamp(input) => {
                write!(f, "{}", input.to_string())
            }
        }
    }
}

impl Display for TimeBucketOffset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeBucketOffset::Date(input) => {
                write!(f, "DATE {}", input.to_string())
            }
            TimeBucketOffset::Interval(input) => {
                write!(f, "INTERVAL {}", input.to_string())
            }
        }
    }
}

fn create_time_bucket(
    bucket_width: Interval,
    input: TimeBucketInput,
    offset: Option<TimeBucketOffset>,
) -> String {
    if let Some(bucket_offset) = offset {
        format!(
            "SELECT time_bucket(INTERVAL {}, {}, {});",
            bucket_width, input, bucket_offset
        )
    } else {
        format!("SELECT time_bucket(INTERVAL {}, {});", bucket_width, input)
    }
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_date_no_offset(
    bucket_width: Interval,
    input: Date,
) -> TableIterator<'static, (name!(time_bucket, Date),)> {
    let bucket_query = create_time_bucket(bucket_width, TimeBucketInput::Date(input), None);

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_date_offset_date(
    bucket_width: Interval,
    input: Date,
    offset: Date,
) -> TableIterator<'static, (name!(time_bucket, Date),)> {
    let bucket_query = create_time_bucket(
        bucket_width,
        TimeBucketInput::Date(input),
        Some(TimeBucketOffset::Date(offset)),
    );

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_date_offset_interval(
    bucket_width: Interval,
    input: Date,
    offset: Interval,
) -> TableIterator<'static, (name!(time_bucket, Date),)> {
    let bucket_query = create_time_bucket(
        bucket_width,
        TimeBucketInput::Date(input),
        Some(TimeBucketOffset::Interval(offset)),
    );

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_timestamp(
    bucket_width: Interval,
    input: Timestamp,
) -> TableIterator<'static, (name!(time_bucket, Timestamp),)> {
    let bucket_query = create_time_bucket(bucket_width, TimeBucketInput::Timestamp(input), None);

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_timestamp_offset_date(
    bucket_width: Interval,
    input: Timestamp,
    offset: Date,
) -> TableIterator<'static, (name!(time_bucket, Timestamp),)> {
    let bucket_query = create_time_bucket(
        bucket_width,
        TimeBucketInput::Timestamp(input),
        Some(TimeBucketOffset::Date(offset)),
    );

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}

#[pg_extern(name = "time_bucket")]
pub fn time_bucket_timestamp_offset_interval(
    bucket_width: Interval,
    input: Timestamp,
    offset: Interval,
) -> TableIterator<'static, (name!(time_bucket, Timestamp),)> {
    let bucket_query = create_time_bucket(
        bucket_width,
        TimeBucketInput::Timestamp(input),
        Some(TimeBucketOffset::Interval(offset)),
    );

    TableIterator::once((bucket_query
        .parse()
        .unwrap_or_else(|err| panic!("There was an error while parsing time_bucket(): {}", err)),))
}
