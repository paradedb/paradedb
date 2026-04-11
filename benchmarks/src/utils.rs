// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use anyhow::{bail, Context, Result};
use duckdb::{AccessMode, Config, Connection};

pub fn open_duckdb_conn() -> Result<Connection> {
    let config = Config::default()
        .access_mode(AccessMode::Automatic)?
        .enable_autoload_extension(true)?;
    let conn = Connection::open_in_memory_with_flags(config)
        .context("Failed to open DuckDB in-memory connection")?;

    conn.execute_batch(
        "CREATE OR REPLACE SECRET secret (TYPE s3, PROVIDER credential_chain);",
    )
    .context("Failed to configure S3 credentials. Ensure AWS credentials are available via environment variables, ~/.aws/credentials, or instance metadata.")?;

    conn.execute("INSTALL httpfs", [])
        .with_context(|| "Failed to install httpfs extension")?;
    conn.execute("LOAD httpfs", [])
        .with_context(|| "Failed to load httpfs extension")?;
    // Increase timeout (default is 30 seconds) to allow for working with larger files (200MB+)
    conn.execute("SET http_timeout = 120", [])
        .with_context(|| "Failed to configure http timeout")?;

    Ok(conn)
}

/// check that each table has at least one parquet file, and that the output location for each
/// table is empty
pub fn validate_input_output(
    tables: &[String],
    conn: &Connection,
    input: &str,
    output: &str,
) -> Result<()> {
    println!("Validating input and output paths...");
    let mut missing_tables: Vec<String> = Vec::new();
    let mut filled_outputs: Vec<String> = Vec::new();

    for table in tables.iter() {
        let input_glob = format!("{input}/{table}/*.parquet");
        let input_file_count: usize = conn
            .query_row(
                &format!("SELECT count(*) FROM (SELECT * FROM glob('{input_glob}') LIMIT 1)"),
                [],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to check parquet files for table '{table}'"))?;
        let input_exists = input_file_count > 0;

        let output_glob = format!("{output}/{table}/*");
        let output_file_count: usize = conn
            .query_row(
                &format!("SELECT count(*) FROM (SELECT * FROM glob('{output_glob}') LIMIT 1)"),
                [],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to check output files for table '{table}'"))?;
        let output_empty = output_file_count == 0;

        if !input_exists {
            println!("  {table}: no parquet files found at '{input_glob}'");
            missing_tables.push(table.clone());
        }
        if !output_empty {
            println!("  {table}: output directory not empty '{output_glob}'");
            filled_outputs.push(table.clone());
        }
        if input_exists && output_empty {
            println!("  {table}: ok");
        }
    }

    match (missing_tables.is_empty(), filled_outputs.is_empty()) {
        (false, false) => bail!(
            "No parquet files found for {} table(s): {}.\nOutput directories not empty for {} table(s): {}\nAborting before any conversion work.",
            filled_outputs.len(),
            filled_outputs.join(", "),
            missing_tables.len(),
            missing_tables.join(", ")
        ),
        (false, true) => bail!(
            "No parquet files found for {} table(s): {}. Aborting before any conversion work.",
            missing_tables.len(),
            missing_tables.join(", ")
        ),
        (true, false) => bail!(
            "Output directories not empty for {} table(s): {} Aborting before any conversion work.",
            filled_outputs.len(),
            filled_outputs.join(", "),
        ),
        (true, true) => {}
    }

    Ok(())
}
