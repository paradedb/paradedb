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

use anyhow::Result;
use datafusion::parquet::arrow::ArrowWriter;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{
    primitive_record_batch, primitive_setup_fdw_local_file_listing, record_batch_with_casing,
    setup_local_file_listing_with_casing,
};
use shared::fixtures::tempfile::TempDir;
use sqlx::{FromRow, PgConnection};
use std::fs::File;

#[derive(FromRow, Debug)]
struct ForeignTableInformationSchema {
    foreign_table_name: String,
}

#[rstest]
async fn test_table_case_sensitivity(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(
        parquet_path.as_path().to_str().unwrap(),
        "\"PrimitiveTable\"",
    )
    .execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM \"PrimitiveTable\"".fetch_recordbatch(&mut conn, &stored_batch.schema());

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    let retrieved_batch = "SELECT * FROM public.\"PrimitiveTable\""
        .fetch_recordbatch(&mut conn, &stored_batch.schema());

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    let retrieved_batch = "SELECT * FROM \"public\".\"PrimitiveTable\""
        .fetch_recordbatch(&mut conn, &stored_batch.schema());

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    Ok(())
}

#[rstest]
async fn test_reserved_table_name(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    match primitive_setup_fdw_local_file_listing(
        parquet_path.as_path().to_str().unwrap(),
        "duckdb_types",
    )
    .execute_result(&mut conn)
    {
        Ok(_) => {
            panic!("should have failed to create table with reserved name")
        }
        Err(e) => {
            assert_eq!(e.to_string(), "error returned from database: Table name 'duckdb_types' is not allowed because it is reserved by DuckDB")
        }
    }

    Ok(())
}

#[rstest]
async fn test_invalid_file(mut conn: PgConnection) -> Result<()> {
    match primitive_setup_fdw_local_file_listing("invalid_file.parquet", "primitive")
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("should have failed to create table with invalid file"),
        Err(e) => {
            assert_eq!(
                e.to_string(),
                "error returned from database: IO Error: No files found that match the pattern \"invalid_file.parquet\""
            )
        }
    }

    Ok(())
}

#[rstest]
async fn test_recreated_view(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "primitive")
        .execute(&mut conn);

    "DROP FOREIGN TABLE primitive".execute(&mut conn);
    format!(
        "CREATE FOREIGN TABLE primitive (id INT) SERVER parquet_server OPTIONS (files '{}')",
        parquet_path.to_str().unwrap()
    )
    .execute(&mut conn);

    Ok(())
}

#[rstest]
async fn test_preserve_casing(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = record_batch_with_casing()?;
    let parquet_path = tempdir.path().join("test_casing.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    setup_local_file_listing_with_casing(parquet_path.as_path().to_str().unwrap(), "primitive")
        .execute(&mut conn);

    let retrieved_batch: Vec<(bool,)> = "SELECT \"Boolean_Col\" FROM primitive".fetch(&mut conn);
    assert_eq!(retrieved_batch.len(), 3);

    Ok(())
}

#[rstest]
async fn test_preserve_casing_on_table_create(
    mut conn: PgConnection,
    tempdir: TempDir,
) -> Result<()> {
    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "MyTable")
        .execute(&mut conn);

    format!(
        "CREATE FOREIGN TABLE \"MyTable\" () SERVER parquet_server OPTIONS (files '{}', preserve_casing 'true')",
        parquet_path.to_str().unwrap()
    )
        .execute(&mut conn);

    match "SELECT foreign_table_name FROM information_schema.foreign_tables WHERE foreign_table_name='MyTable';".fetch_result::<ForeignTableInformationSchema>(&mut conn) {
        Ok(result) => {
            if let Some(mytable) = result.iter().find(|name| name.foreign_table_name == "MyTable") {
                assert_eq!(mytable.foreign_table_name, "MyTable");
                assert_ne!(mytable.foreign_table_name, "mytable");
            } else {
                panic!("should have successfully found table with name \"MyTable\"");
            };
        }
        Err(error) => {
            panic!("should have successfully found table with name \"MyTable\": {}", error.to_string());
        }
    }

    match "SELECT * FROM \"MyTable\"".execute_result(&mut conn) {
        Ok(_) => {}
        Err(error) => {
            panic!(
                "should have successfully queried case sensitive table \"MyTable\": {}",
                error.to_string()
            );
        }
    }

    Ok(())
}
