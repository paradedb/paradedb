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
use shared::fixtures::arrow::{primitive_record_batch, primitive_setup_fdw_local_file_listing};
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;
use std::fs::File;

#[rstest]
async fn test_time_bucket(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
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

    let foreign_table_name: Vec<(String,)> =
        "SELECT foreign_table_name FROM information_schema.foreign_tables WHERE foreign_table_name='MyTable';".fetch(&mut conn);
    assert_eq!(foreign_table_name.len(), 1);
    assert_ne!(foreign_table_name[0].0, "mytable");
    assert_eq!(foreign_table_name[0].0, "MyTable");

    match "SELECT * FROM \"MyTable\"".execute_result(&mut conn) {
        Ok(_) => {}
        Err(error) => {
            panic!(
                "should have successfully queried case sensitive table \"MyTable\": {}",
                error
            );
        }
    }

    Ok(())
}
