mod fixtures;

use std::fs::File;

use anyhow::Result;
use datafusion::parquet::arrow::ArrowWriter;
use deltalake::operations::create::CreateBuilder;
use deltalake::writer::{DeltaWriter, RecordBatchWriter};
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{
    delta_primitive_record_batch, primitive_record_batch, primitive_setup_fdw_local_file_delta,
    primitive_setup_fdw_local_file_listing, primitive_setup_fdw_s3_delta,
    primitive_setup_fdw_s3_listing,
};
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;

#[rstest]
async fn test_trip_count(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    let s3_bucket = "test-trip-setup";
    let s3_key = "test_trip_setup.parquet";
    let s3_endpoint = s3.url.clone();
    let s3_object_path = format!("s3://{s3_bucket}/{s3_key}");

    NycTripsTable::setup().execute(&mut conn);
    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    s3.client.create_bucket().bucket(s3_bucket).send().await?;
    s3.create_bucket(s3_bucket).await?;
    s3.put_rows(s3_bucket, s3_key, &rows).await?;

    NycTripsTable::setup_s3_listing_fdw(&s3_endpoint, &s3_object_path).execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM trips".fetch_one(&mut conn);

    assert_eq!(count.0, 100);

    Ok(())
}

#[rstest]
async fn test_arrow_types_s3_listing(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    let s3_bucket = "test-arrow-types-s3-listing";
    let s3_key = "test_arrow_types.parquet";
    let s3_endpoint = s3.url.clone();
    let s3_object_path = format!("s3://{s3_bucket}/{s3_key}");

    let stored_batch = primitive_record_batch()?;
    s3.create_bucket(s3_bucket).await?;
    s3.put_batch(s3_bucket, s3_key, &stored_batch).await?;

    primitive_setup_fdw_s3_listing(&s3_endpoint, &s3_object_path, "parquet").execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM primitive".fetch_recordbatch(&mut conn, &stored_batch.schema());

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());
    for field in stored_batch.schema().fields() {
        assert_eq!(
            stored_batch.column_by_name(field.name()),
            retrieved_batch.column_by_name(field.name())
        )
    }

    Ok(())
}

#[rstest]
async fn test_arrow_types_s3_delta(
    #[future(awt)] s3: S3,
    mut conn: PgConnection,
    tempdir: TempDir,
) -> Result<()> {
    let s3_bucket = "test-arrow-types-s3-delta";
    let s3_key = "test_arrow_types.parquet";
    let s3_endpoint = s3.url.clone();
    let s3_object_path = format!("s3://{s3_bucket}/{s3_key}");
    let temp_path = tempdir.path();

    let batch = delta_primitive_record_batch()?;

    let delta_schema = deltalake::kernel::Schema::try_from(batch.schema().as_ref())?;
    let mut table = CreateBuilder::new()
        .with_location(temp_path.to_string_lossy())
        .with_columns(delta_schema.fields().to_vec())
        .await?;
    let mut writer = RecordBatchWriter::for_table(&table)?;
    writer.write(batch).await?;
    writer.flush_and_commit(&mut table).await?;

    s3.create_bucket(s3_bucket).await?;
    s3.put_directory(s3_bucket, temp_path).await?;

    primitive_setup_fdw_s3_delta(&s3_endpoint, &s3_object_path, "parquet").execute(&mut conn);

    // let retrieved_batch =
    //     "SELECT * FROM primitive".fetch_recordbatch(&mut conn, &stored_batch.schema());

    // assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());
    // for field in stored_batch.schema().fields() {
    //     assert_eq!(
    //         stored_batch.column_by_name(field.name()),
    //         retrieved_batch.column_by_name(field.name())
    //     )
    // }

    Ok(())
}

#[rstest]
async fn test_arrow_types_local_file_listing(
    mut conn: PgConnection,
    tempdir: TempDir,
) -> Result<()> {
    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.to_str().unwrap(), "parquet")
        .execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM primitive".fetch_recordbatch(&mut conn, &stored_batch.schema());

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());
    for field in stored_batch.schema().fields() {
        assert_eq!(
            stored_batch.column_by_name(field.name()),
            retrieved_batch.column_by_name(field.name())
        )
    }

    Ok(())
}

#[rstest]
async fn test_arrow_types_local_file_delta(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    // let temp_path = tempdir.path().to_string_lossy();
    let temp_path = tempdir.into_path();
    let batch = delta_primitive_record_batch()?;
    println!("TEMP: {temp_path:?}");
    let delta_schema = deltalake::kernel::Schema::try_from(batch.schema().as_ref())?;
    let mut table = CreateBuilder::new()
        .with_location(temp_path.to_string_lossy().as_ref())
        .with_columns(delta_schema.fields().to_vec())
        .await?;
    let mut writer = RecordBatchWriter::for_table(&table)?;
    writer.write(batch.clone()).await?;
    writer.flush_and_commit(&mut table).await?;

    primitive_setup_fdw_local_file_delta(&temp_path.to_string_lossy(), "parquet")
        .execute(&mut conn);

    let rows = "SELECT * FROM primitive".fetch_dynamic(&mut conn);
    println!("{}", rows.len());
    // let retrieved_batch = "SELECT * FROM primitive".fetch_recordbatch(&mut conn, &batch.schema());

    // assert_eq!(batch.num_columns(), retrieved_batch.num_columns());
    // for field in batch.schema().fields() {
    //     assert_eq!(
    //         batch.column_by_name(field.name()),
    //         retrieved_batch.column_by_name(field.name())
    //     )
    // }

    Ok(())
}
