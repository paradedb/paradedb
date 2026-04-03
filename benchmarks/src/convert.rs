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
use clap::Parser;
use duckdb::{AccessMode, Config, Connection};

#[derive(Parser)]
pub struct ConvertArgs {
    /// Input S3 path to the dataset (e.g. s3://bucket/path/to/dataset).
    /// Each table is a subdirectory containing partitioned parquet files.
    #[arg(long)]
    pub input: String,

    /// Output S3 path for the converted CSV files (e.g. s3://bucket/path/to/output).
    /// CSV files will be written to subdirectories matching the table names.
    #[arg(long)]
    pub output: String,

    /// Comma-separated list of table names to convert.
    /// Each table name corresponds to a subdirectory under the input path.
    #[arg(long, required = true, value_delimiter = ',')]
    pub tables: Vec<String>,

    /// Validate inputs and list files that would be converted, without performing conversion.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

fn open_duckdb_conn() -> Result<Connection> {
    let config = Config::default()
        .access_mode(AccessMode::Automatic)?
        .enable_autoload_extension(true)?;
    let conn = Connection::open_in_memory_with_flags(config)
        .context("Failed to open DuckDB in-memory connection")?;

    conn.execute_batch(
        "CREATE OR REPLACE SECRET secret (TYPE s3, PROVIDER credential_chain);",
    )
    .context("Failed to configure S3 credentials. Ensure AWS credentials are available via environment variables, ~/.aws/credentials, or instance metadata.")?;

    Ok(conn)
}

pub fn run_convert(args: ConvertArgs) -> Result<()> {
    let conn = open_duckdb_conn()?;

    let input = args.input.trim_end_matches('/');
    let output = args.output.trim_end_matches('/');

    // Validation phase: check that each table has at least one parquet file.
    println!("Validating input paths...");
    let mut missing_tables: Vec<String> = Vec::new();

    for table in &args.tables {
        let glob_pattern = format!("{input}/{table}/*.parquet");
        let exists: bool = conn
            .query_row(
                &format!("SELECT count(*) > 0 FROM (SELECT * FROM glob('{glob_pattern}') LIMIT 1)"),
                [],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to check parquet files for table '{table}'"))?;

        if exists {
            println!("  {table}: ok");
        } else {
            println!("  {table}: no parquet files found at '{glob_pattern}'");
            missing_tables.push(table.clone());
        }
    }

    if !missing_tables.is_empty() {
        bail!(
            "No parquet files found for {} table(s): {}. Aborting before any conversion work.",
            missing_tables.len(),
            missing_tables.join(", ")
        );
    }

    if args.dry_run {
        println!("\nDry run: counting planned conversions...");
        for table in &args.tables {
            let glob_pattern = format!("{input}/{table}/*.parquet");
            let mut stmt = conn
                .prepare(&format!("SELECT count(*) FROM glob('{glob_pattern}')"))
                .with_context(|| {
                    format!("Failed to prepare query to count parquet files for table '{table}'")
                })?;
            let count: usize = stmt
                .query_one([], |row| row.get(0))
                .with_context(|| format!("Failed to count parquet files for table '{table}'"))?;
            println!("  Table '{table}' ({count} file(s)):");
        }
        println!("\nDry run complete. No files were converted.");
        return Ok(());
    }

    // Setup phase
    conn.execute("INSTALL httpfs", [])
        .with_context(|| "Failed to install httpfs extension")?;
    conn.execute("LOAD httpfs", [])
        .with_context(|| "Failed to load httpfs extension")?;
    // Increase timeout (default is 30 seconds) to allow for working with larger files (200MB+)
    conn.execute("SET http_timeout = 120", [])
        .with_context(|| "Failed to configure http timeout")?;

    // Conversion phase: one COPY per table, DuckDB handles parallelism internally.
    println!("\nConverting parquet to CSV...");
    for table in &args.tables {
        println!("  Converting table '{table}'...");
        let sql = format!(
            "COPY (SELECT * FROM read_parquet('{input}/{table}/*.parquet')) \
             TO '{output}/{table}' (FORMAT CSV, HEADER true, PER_THREAD_OUTPUT true);"
        );

        conn.execute_batch(&sql)
            .with_context(|| format!("Failed to convert table '{table}'"))?;

        println!("  {table}: done");
    }

    println!("\nVerifying input and output table row counts...");
    for table in &args.tables {
        println!("  Checking table '{table}'...");

        let mut parquet_stmt = conn
            .prepare(&format!(
                "SELECT count(*) FROM read_parquet('{input}/{table}/*.parquet')",
            ))
            .with_context(|| format!("Failed to prepare parquet count statement for {table}"))?;
        let parquet_count: usize = parquet_stmt
            .query_one([], |row| row.get(0))
            .with_context(|| format!("Failed to query parquet count for {table}"))?;

        let mut csv_stmt = conn
            .prepare(&format!(
                "SELECT count(*) FROM read_csv('{output}/{table}/*.csv', parallel=false, header=true)",
            ))
            .with_context(|| format!("Failed to prepare csv count statement for {table}"))?;
        let csv_count: usize = csv_stmt
            .query_one([], |row| row.get(0))
            .with_context(|| format!("Failed to query csv count for {table}"))?;

        println!("  {parquet_count} -> {csv_count}");
        if parquet_count != csv_count {
            bail!("{parquet_count} rows for {table} exist in the source, but only {csv_count} were found in the output");
        }
    }

    println!("\nConversion complete.");
    Ok(())
}
