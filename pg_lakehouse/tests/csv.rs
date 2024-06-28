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
use datafusion::arrow::array::RecordBatchWriter;
use datafusion::common::arrow::csv::WriterBuilder;
use fixtures::*;
use rstest::*;
use shared::fixtures::arrow::{flights_record_batch_csv, flights_setup_fdw_csv_local};
use shared::fixtures::tempfile::TempDir;
use sqlx::PgConnection;
use std::fs::File;

#[rstest]
async fn test_csv_local_file(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = flights_record_batch_csv()?;
    let csv_path = tempdir.path().join("flights.csv");
    let csv_file = File::create(&csv_path)?;

    let mut writer = WriterBuilder::new().with_header(true).build(csv_file);
    writer.write(&stored_batch)?;
    writer.close()?;

    flights_setup_fdw_csv_local(csv_path.as_path().to_str().unwrap(), "flights").execute(&mut conn);

    let retrieved_batch =
        "SELECT * FROM flights".fetch_recordbatch(&mut conn, &stored_batch.schema());

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
async fn test_sniff_csv(mut conn: PgConnection, tempdir: TempDir) -> Result<()> {
    let stored_batch = flights_record_batch_csv()?;
    let csv_path = tempdir.path().join("flights.csv");
    let csv_file = File::create(&csv_path)?;

    let mut writer = WriterBuilder::new().with_header(true).build(csv_file);
    writer.write(&stored_batch)?;
    writer.close()?;

    flights_setup_fdw_csv_local(csv_path.as_path().to_str().unwrap(), "flights").execute(&mut conn);

    let columns: (String,) = format!(
        "SELECT columns FROM sniff_csv('{}')",
        csv_path.as_path().to_str().unwrap()
    )
    .fetch_one(&mut conn);

    assert_eq!(
        columns.0,
        "[{'name': flight_date, 'type': DATE}, {'name': carrier, 'type': VARCHAR}]"
    );

    Ok(())
}
