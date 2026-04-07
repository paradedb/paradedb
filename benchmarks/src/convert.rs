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

use crate::duckdb_utils::open_duckdb_conn;
use duckdb::Connection;

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

/// Validation phase:
/// check that each table has at least one parquet file, and that the output location for each
/// table is empty
fn validate_conversion(
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

pub fn run_convert(args: ConvertArgs) -> Result<()> {
    let conn = open_duckdb_conn()?;

    let input = args.input.trim_end_matches('/');
    let output = args.output.trim_end_matches('/');

    validate_conversion(&args.tables, &conn, input, output)?;

    if args.dry_run {
        println!("\nDry run: counting planned conversions...");
        for table in &args.tables {
            let glob_pattern = format!("{input}/{table}/*.parquet");
            let count: usize = conn
                .query_row(
                    &format!("SELECT count(*) FROM glob('{glob_pattern}')"),
                    [],
                    |row| row.get(0),
                )
                .with_context(|| {
                    format!("Failed to execute query to count parquet files for table '{table}'")
                })?;
            println!("  Table '{table}' ({count} file(s)):");
        }
        println!("\nDry run complete. No files were converted.");
        return Ok(());
    }

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

        let parquet_count: usize = conn
            .query_row(
                &format!("SELECT count(*) FROM read_parquet('{input}/{table}/*.parquet')",),
                [],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to query parquet row count for {table}"))?;

        let csv_count: usize = conn
            .query_row(&format!(
                "SELECT count(*) FROM read_csv('{output}/{table}/*.csv', parallel=false, header=true)",
            ), [], |row| row.get(0))
            .with_context(|| format!("Failed to query csv row count for {table}"))?;

        println!("  {parquet_count} -> {csv_count}");
        if parquet_count != csv_count {
            bail!("{parquet_count} rows for {table} exist in the source, but only {csv_count} were found in the output");
        }
    }

    println!("\nConversion complete.");
    Ok(())
}
