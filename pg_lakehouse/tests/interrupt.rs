mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::primitive_setup_fdw_local_file_listing;
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;

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

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "parquet")
        .execute(&mut first_conn);

    let retrieved_batch = "SELECT pg_sleep(60), * FROM primitive"
        .fetch_recordbatch(&mut first_conn, &stored_batch.schema());
}
