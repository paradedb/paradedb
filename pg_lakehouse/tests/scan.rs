mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn test_trip_setup(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    let s3_bucket = "test-trip-setup";
    let s3_key = "test_trip_setup.parquet";
    let s3_endpoint = s3.url.clone();
    let s3_object_path = format!("s3://{s3_bucket}/{s3_key}");

    NycTripsTable::setup().execute(&mut conn);
    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    s3.client.create_bucket().bucket(s3_bucket).send().await?;
    s3.create_bucket(s3_bucket).await?;
    s3.put(s3_bucket, s3_key, &rows).await?;

    NycTripsTable::setup_fdw(&s3_endpoint, s3_bucket, &s3_object_path).execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM trips".fetch_one(&mut conn);

    assert_eq!(count.0, 100);

    Ok(())
}
