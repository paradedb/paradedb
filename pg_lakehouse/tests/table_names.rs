mod fixtures;

use anyhow::Result;
use datafusion::parquet::arrow::ArrowWriter;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{primitive_record_batch, primitive_setup_fdw_local_file_listing};
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;
use std::fs::File;

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
        "parquet",
        "\"PrimitiveTable\"",
    )
    .execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM \"PrimitiveTable\"".fetch_recordbatch(&mut conn, &stored_batch.schema())?;

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    let retrieved_batch = "SELECT * FROM public.\"PrimitiveTable\""
        .fetch_recordbatch(&mut conn, &stored_batch.schema())?;

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    let retrieved_batch = "SELECT * FROM \"public\".\"PrimitiveTable\""
        .fetch_recordbatch(&mut conn, &stored_batch.schema())?;

    assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());

    Ok(())
}
