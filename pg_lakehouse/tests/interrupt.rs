mod fixtures;

use anyhow::Result;
use datafusion::parquet::arrow::ArrowWriter;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{primitive_record_batch, primitive_setup_fdw_local_file_listing};
use shared::fixtures::tempfile::TempDir;
use std::fs::File;

#[rstest]
async fn test_sigint(database: Db, tempdir: TempDir) -> Result<()> {
    let mut first_conn = database.connection().await;
    let mut second_conn = database.connection().await;

    sqlx::query("CREATE EXTENSION pg_lakehouse;")
        .execute(&mut first_conn)
        .await
        .expect("could not create extension pg_lakehouse");

    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;
    let schema = stored_batch.schema();

    let mut writer = ArrowWriter::try_new(parquet_file, schema.clone(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "parquet")
        .execute(&mut first_conn);

    let retrieved_batch =
        std::thread::spawn(move || {
            match "SELECT pg_sleep(600), * FROM primitive"
                .fetch_recordbatch(&mut first_conn, &schema)
            {
                Err(err) => assert_eq!(
                    err.to_string(),
                    "error returned from database: canceling statement due to user request"
                ),
                Ok(_) => panic!("expected query to be interrupted"),
            }
        });

    let cancel_request = std::thread::spawn(move || {
        let (pg_backend_pid,): (i32,) = "SELECT pid FROM pg_stat_activity WHERE query = 'SELECT pg_sleep(600), * FROM primitive'".fetch_one(&mut second_conn);
        format!("SELECT pg_cancel_backend({pg_backend_pid})")
            .execute_result(&mut second_conn)
            .unwrap();
    });

    cancel_request.join().unwrap();
    retrieved_batch.join().unwrap();

    Ok(())
}

#[rstest]
async fn test_sigkill(database: Db, tempdir: TempDir) -> Result<()> {
    let mut first_conn = database.connection().await;
    let mut second_conn = database.connection().await;

    sqlx::query("CREATE EXTENSION pg_lakehouse;")
        .execute(&mut first_conn)
        .await
        .expect("could not create extension pg_lakehouse");

    let stored_batch = primitive_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;
    let schema = stored_batch.schema();

    let mut writer = ArrowWriter::try_new(parquet_file, schema.clone(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "parquet")
        .execute(&mut first_conn);

    let (pg_backend_pid,): (i32,) = "SELECT pg_backend_pid()".fetch_one(&mut first_conn);

    let retrieved_batch =
        std::thread::spawn(move || {
            match "SELECT pg_sleep(600), * FROM primitive"
                .fetch_recordbatch(&mut first_conn, &schema)
            {
                Err(err) => assert_eq!(
                err.to_string(),
                "error returned from database: terminating connection due to administrator command"
            ),
                Ok(_) => panic!("expected query to be interrupted"),
            }
        });

    let cancel_request = std::thread::spawn(move || {
        format!("SELECT pg_terminate_backend({pg_backend_pid})")
            .execute_result(&mut second_conn)
            .unwrap();
    });

    cancel_request.join().unwrap();
    retrieved_batch.join().unwrap();

    Ok(())
}
