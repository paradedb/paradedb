//time_bucket(bucket_width, date[, offset])
// Description	Truncate date by the specified interval bucket_width. Buckets are offset by offset interval.
// Example	time_bucket(INTERVAL '2 months', DATE '1992-04-20', INTERVAL '1 month')
// Result	1992-04-01
// time_bucket(bucket_width, date[, origin])
// Description	Truncate date by the specified interval bucket_width. Buckets are aligned relative to origin date. origin defaults to 2000-01-03 for buckets that don't include a month or year interval, and to 2000-01-01 for month and year buckets.
// Example	time_bucket(INTERVAL '2 weeks', DATE '1992-04-20', DATE '1992-04-01')
// Result	1992-04-15

use pgrx::*;
use sqlx::postgres::PgColumn;
use sqlx::postgres::types::PgInterval;

struct TimeBucketOptions {
    bucket_width: PgInterval,
    column: PgColumn,
}

enum TimeBucketType {
    Origin,
    Offset
}

#[pg_extern]
fn time_bucket() {}

#[cfg(test)]
mod tests {

    #[test]
    fn test_create_time_bucket() {}
}
