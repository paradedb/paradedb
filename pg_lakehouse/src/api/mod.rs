use chrono::Datelike;
use pgrx::*;

#[pg_extern]
pub fn to_date(days: i64) -> datum::Date {
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let date = epoch + chrono::Duration::days(days);

    datum::Date::new(date.year(), date.month() as u8, date.day() as u8).unwrap()
}
