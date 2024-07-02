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

const S3_TRIPS_BUCKET: &str = "test-trip-setup";
const S3_TRIPS_KEY: &str = "test_trip_setup.parquet";

#[rstest]
async fn test_trip_count(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    NycTripsTable::setup().execute(&mut conn);
    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    s3.client
        .create_bucket()
        .bucket(S3_TRIPS_BUCKET)
        .send()
        .await?;
    s3.create_bucket(S3_TRIPS_BUCKET).await?;
    s3.put_rows(S3_TRIPS_BUCKET, S3_TRIPS_KEY, &rows).await?;

    NycTripsTable::setup_s3_listing_fdw(
        &s3.url.clone(),
        &format!("s3://{S3_TRIPS_BUCKET}/{S3_TRIPS_KEY}"),
    )
    .execute(&mut conn);

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

    primitive_setup_fdw_s3_listing(&s3_endpoint, &s3_object_path, "primitive").execute(&mut conn);

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
#[ignore = "bug in DuckDB delta_scan over custom endpoints"]
async fn test_arrow_types_s3_delta(
    #[future(awt)] s3: S3,
    mut conn: PgConnection,
    tempdir: TempDir,
) -> Result<()> {
    let s3_bucket = "test-arrow-types-s3-delta";
    let s3_path = "test_arrow_types";
    let s3_endpoint = s3.url.clone();
    let s3_object_path = format!("s3://{s3_bucket}/{s3_path}");
    let temp_path = tempdir.path();

    let batch = delta_primitive_record_batch()?;

    let delta_schema = deltalake::kernel::Schema::try_from(batch.schema().as_ref())?;
    let mut table = CreateBuilder::new()
        .with_location(temp_path.to_string_lossy())
        .with_columns(delta_schema.fields().to_vec())
        .await?;
    let mut writer = RecordBatchWriter::for_table(&table)?;
    writer.write(batch.clone()).await?;
    writer.flush_and_commit(&mut table).await?;

    s3.create_bucket(s3_bucket).await?;
    s3.put_directory(s3_bucket, s3_path, temp_path).await?;

    primitive_setup_fdw_s3_delta(&s3_endpoint, &s3_object_path, "delta_primitive")
        .execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM delta_primitive".fetch_recordbatch(&mut conn, &batch.schema());

    assert_eq!(batch.num_columns(), retrieved_batch.num_columns());
    for field in batch.schema().fields() {
        assert_eq!(
            batch.column_by_name(field.name()),
            retrieved_batch.column_by_name(field.name())
        )
    }

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

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "primitive")
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
    let temp_path = tempdir.path();
    let batch = delta_primitive_record_batch()?;
    let delta_schema = deltalake::kernel::Schema::try_from(batch.schema().as_ref())?;
    let mut table = CreateBuilder::new()
        .with_location(temp_path.to_string_lossy().as_ref())
        .with_columns(delta_schema.fields().to_vec())
        .await?;
    let mut writer = RecordBatchWriter::for_table(&table)?;
    writer.write(batch.clone()).await?;
    writer.flush_and_commit(&mut table).await?;

    primitive_setup_fdw_local_file_delta(&temp_path.to_string_lossy(), "delta_primitive")
        .execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM delta_primitive".fetch_recordbatch(&mut conn, &batch.schema());

    assert_eq!(batch.num_columns(), retrieved_batch.num_columns());
    for field in batch.schema().fields() {
        assert_eq!(
            batch.column_by_name(field.name()),
            retrieved_batch.column_by_name(field.name())
        )
    }

    Ok(())
}

#[rstest]
async fn test_duckdb_types_parquet_local(
    mut conn: PgConnection,
    tempdir: TempDir,
    duckdb_conn: duckdb::Connection,
) -> Result<()> {
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");

    duckdb_conn
        .execute(&DuckdbTypesTable::create_duckdb_table(), [])
        .unwrap();

    duckdb_conn
        .execute(&DuckdbTypesTable::populate_duckdb_table(), [])
        .unwrap();

    duckdb_conn
        .execute(
            &DuckdbTypesTable::export_duckdb_table(parquet_path.to_str().unwrap()),
            [],
        )
        .unwrap();

    DuckdbTypesTable::create_foreign_table(parquet_path.to_str().unwrap()).execute(&mut conn);
    let row: Vec<DuckdbTypesTable> = format!("SELECT * FROM duckdb_types_test").fetch(&mut conn);

    Ok(())
}
