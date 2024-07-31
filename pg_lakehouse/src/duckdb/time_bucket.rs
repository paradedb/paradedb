//time_bucket(bucket_width, date[, offset])
// Description	Truncate date by the specified interval bucket_width. Buckets are offset by offset interval.
// Example	time_bucket(INTERVAL '2 months', DATE '1992-04-20', INTERVAL '1 month')
// Result	1992-04-01
// time_bucket(bucket_width, date[, origin])
// Description	Truncate date by the specified interval bucket_width. Buckets are aligned relative to origin date. origin defaults to 2000-01-03 for buckets that don't include a month or year interval, and to 2000-01-01 for month and year buckets.
// Example	time_bucket(INTERVAL '2 weeks', DATE '1992-04-20', DATE '1992-04-01')
// Result	1992-04-15
//
use pgrx::*;

#[pg_extern]
pub fn time_bucket(param: &str) -> String {
    log!("Logging out to psql maybe?");
    log!("{}", param);

    "Ok!".to_string()
}
