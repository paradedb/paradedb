mod fixtures;

use anyhow::Result;
use datafusion::{arrow::datatypes::FieldRef, parquet::arrow::ArrowWriter};
use fixtures::*;
use rstest::*;
use serde_arrow::schema::{SchemaLike, TracingOptions};
use sqlx::PgConnection;
use std::fs::File;

#[rstest]
async fn main(#[future(awt)] s3: S3) -> Result<()> {
    s3.client
        .create_bucket()
        .bucket("example-bucket")
        .send()
        .await?;

    let list_buckets_output = s3.client.list_buckets().send().await?;
    assert!(list_buckets_output.buckets.is_some());
    let buckets_list = list_buckets_output.buckets.unwrap();
    assert_eq!(1, buckets_list.len());
    assert_eq!("example-bucket", buckets_list[0].name.as_ref().unwrap());

    Ok(())
}

#[rstest]
async fn test_trip_setup(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    NycTripsTable::setup().execute(&mut conn);

    let bucket = "test-trip-setup";
    let key = "test_trip_setup.parquet";
    s3.client.create_bucket().bucket(bucket).send().await?;

    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    let fields = Vec::<FieldRef>::from_type::<NycTripsTable>(TracingOptions::default())?;
    let batch = serde_arrow::to_record_batch(&fields, &rows)?;

    let mut buf = vec![];
    // let file = File::create("example.parquet")?;
    let mut writer = ArrowWriter::try_new(&mut buf, batch.schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;

    s3.client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(buf.into())
        .send()
        .await?;

    let s3_url = s3.url;
    let s3_object_path = format!("s3://{bucket}/{key}");

    println!("{s3_url}");
    println!("{s3_object_path}");
    std::thread::sleep(std::time::Duration::from_secs(30));

    Ok(())
}
