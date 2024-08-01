use pgrx::iter::TableIterator;
use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize)]
pub enum TimeBucketInput {
    Date(Date),
    Timestamp(Timestamp),
}

#[derive(Serialize, Deserialize)]
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
        format!("INTERVAL {}, {}, {}", bucket_width, input, bucket_offset)
    } else {
        format!("INTERVAL {}, {}", bucket_width, input)
    }
}

#[pg_extern]
pub fn time_bucket(
    bucket_width: Interval,
    input: Date,
) -> TableIterator<'static, (name!(time_bucket, Date),)> {
    let bucket_query = create_time_bucket(bucket_width, TimeBucketInput::Date(input), None);

    TableIterator::once((bucket_query.parse().unwrap(),))
}
