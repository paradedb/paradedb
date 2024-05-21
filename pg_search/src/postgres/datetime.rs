static MICROSECONDS_IN_SECOND: u32 = 1_000_000;

fn datetime_components_to_tantivy_date(
    ymd: Option<(i32, u8, u8)>,
    hms_micro: (u8, u8, u8, u32),
) -> tantivy::schema::Value {
    let naive_dt = match ymd {
        Some(ymd) => chrono::NaiveDate::from_ymd_opt(ymd.0, ymd.1.into(), ymd.2.into()).unwrap(),
        None => chrono::NaiveDateTime::UNIX_EPOCH.date(),
    }
    .and_hms_micro_opt(
        hms_micro.0.into(),
        hms_micro.1.into(),
        hms_micro.2.into(),
        hms_micro.3 % MICROSECONDS_IN_SECOND,
    )
    .unwrap()
    .and_utc();

    tantivy::schema::Value::Date(tantivy::DateTime::from_timestamp_micros(
        naive_dt.timestamp_micros(),
    ))
}

pub fn pgrx_time_to_tantivy_value(value: pgrx::Time) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_timetz_to_tantivy_value(value: pgrx::TimeWithTimeZone) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(None, (v_h, v_m, v_s, v_ms))
}

pub fn pgrx_date_to_tantivy_value(value: pgrx::Date) -> tantivy::schema::Value {
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (0, 0, 0, 0),
    )
}

pub fn pgrx_timestamp_to_tantivy_value(value: pgrx::Timestamp) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}

pub fn pgrx_timestamptz_to_tantivy_value(
    value: pgrx::TimestampWithTimeZone,
) -> tantivy::schema::Value {
    let (v_h, v_m, v_s, v_ms) = value.to_utc().to_hms_micro();
    datetime_components_to_tantivy_date(
        Some((value.year(), value.month(), value.day())),
        (v_h, v_m, v_s, v_ms),
    )
}
