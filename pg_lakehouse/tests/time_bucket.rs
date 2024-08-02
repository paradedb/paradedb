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
use chrono::NaiveDateTime;
use datafusion::parquet::arrow::ArrowWriter;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{primitive_setup_fdw_local_file_listing, time_series_record_batch};
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;
use std::fs::File;

#[rstest]
async fn test_time_bucket(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = time_series_record_batch()?;
    let parquet_path = tempdir.path().join("test_arrow_types.parquet");
    let parquet_file = File::create(&parquet_path)?;

    let mut writer = ArrowWriter::try_new(parquet_file, stored_batch.schema(), None).unwrap();
    writer.write(&stored_batch)?;
    writer.close()?;

    primitive_setup_fdw_local_file_listing(parquet_path.as_path().to_str().unwrap(), "MyTable")
        .execute(&mut conn);

    format!(
        "CREATE FOREIGN TABLE timeseries () SERVER parquet_server OPTIONS (files '{}')",
        parquet_path.to_str().unwrap()
    )
    .execute(&mut conn);

    match "SELECT time_bucket(INTERVAL '2 DAY', timestamp::DATE) AS bucket, AVG(value) as avg_value FROM timeseries GROUP BY bucket ORDER BY bucket;".execute_result(&mut conn) {
        Ok(_) => {}
        Err(error) => {
            panic!(
                "should have successfully called time_bucket() for timeseries data: {}",
                error
            );
        }
    }

    match "SELECT time_bucket(INTERVAL '2 DAY') AS bucket, AVG(value) as avg_value FROM timeseries GROUP BY bucket ORDER BY bucket;".execute_result(&mut conn) {
        Ok(_) => {
            panic!(
                "should have failed call to time_bucket() for timeseries data with incorrect parameters"
            );
        }
        Err(_) => {}
    }

    let data: Vec<(NaiveDateTime,)> = "SELECT time_bucket(INTERVAL '6 MINUTE', timestamp::TIMESTAMP) AS bucket, AVG(value) as avg_value FROM timeseries GROUP BY bucket ORDER BY bucket;"
        .fetch_result(&mut conn).unwrap();

    assert_eq!(2, data.len());

    let data: Vec<(NaiveDateTime,)> = "SELECT time_bucket(INTERVAL '1 MINUTE', timestamp::TIMESTAMP) AS bucket, AVG(value) as avg_value FROM timeseries GROUP BY bucket ORDER BY bucket;"
        .fetch_result(&mut conn).unwrap();

    assert_eq!(10, data.len());

    Ok(())
}
