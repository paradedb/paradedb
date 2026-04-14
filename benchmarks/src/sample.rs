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
use duckdb::Connection;
use std::time::Instant;

use crate::config::{load_dataset_config, topological_order};
use crate::utils::{open_duckdb_conn, validate_input_output};

#[derive(Parser)]
pub struct SampleArgs {
    /// Input path to the dataset (S3 or local).
    /// Each table is a subdirectory containing parquet files.
    #[arg(long)]
    pub input: String,

    /// Output path for the sampled parquet files (S3 or local).
    /// Parquet files will be written to subdirectories matching the table names.
    #[arg(long)]
    pub output: String,

    /// Path to the TOML config file describing table relationships.
    #[arg(long)]
    pub config: String,

    /// Target number of rows for the root table.
    #[arg(long)]
    pub rows: u64,

    /// Validate and report planned row counts without writing.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

fn parquet_glob_pattern(base: &str, table_name: &str) -> String {
    format!("{base}/{table_name}/*.parquet")
}

fn count_rows(conn: &Connection, glob: &str) -> Result<u64> {
    let sql = format!("SELECT count(*) FROM read_parquet('{glob}')");
    let count: u64 = conn
        .query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("Failed to count rows for '{glob}'"))?;
    Ok(count)
}

pub fn run_sample(args: SampleArgs) -> Result<()> {
    let config = load_dataset_config(&args.config)?;

    let conn = open_duckdb_conn()?;

    let input = args.input.trim_end_matches('/');
    let output = args.output.trim_end_matches('/');

    let table_names: Vec<String> = config.tables.iter().map(|t| t.name.clone()).collect();
    validate_input_output(&table_names, &conn, input, output)?;

    // Determine processing order.
    let order = topological_order(&config)?;

    // Sample the root table.
    let root = &config.tables[order[0]];
    let root_glob = parquet_glob_pattern(input, &root.name);
    let total_rows = count_rows(&conn, &root_glob)?;

    if total_rows == 0 {
        bail!("Root table '{}' has no rows", root.name);
    }

    let target = args.rows;
    if target > total_rows {
        bail!(
            "Target rows ({target}) exceeds total rows ({total_rows}) in root table '{}'",
            root.name
        );
    }

    let local_root_data_path = format!("/tmp/local_source/{}", root.name);
    let local_glob = format!("{local_root_data_path}/*.parquet");

    // copy root table locally to speed up sampling.
    std::fs::create_dir_all(&local_root_data_path)
        .with_context(|| "Failed to make root table data directory")?;
    println!("Copying root table data to local disk...");
    let sql = format!(
        "COPY (SELECT * FROM read_parquet('{}')) \
         TO '{}' (FORMAT PARQUET, OVERWRITE true, PER_THREAD_OUTPUT true)",
        root_glob, local_root_data_path
    );
    conn.execute_batch(&sql)
        .with_context(|| "Failed to copy root table data locally")?;

    // disable multi-threading, required for deterministic output
    // See: https://duckdb.org/docs/current/sql/samples#syntax
    conn.execute("SET threads = 1;", [])
        .with_context(|| "Failed to set thread count")?;

    let percentage = (target as f64 / total_rows as f64) * 100.0;

    // We use reservoir for small sample sizes, since it allows us to be exact. However, it
    // requires materializing the entire sample in memory, so we use the system method for larger
    // counts, which gives us an approximate count (usually within 3-5%).
    let sample_arg = if target <= 100_000 {
        format!("reservoir({target} ROWS)")
    } else {
        format!("system({percentage:.5} PERCENT)")
    };
    let sql = format!(
        "CREATE TABLE sampled_{name} AS \
         SELECT * FROM  read_parquet('{local_path}') \
         USING SAMPLE {sample_arg} REPEATABLE({seed})",
        name = root.name,
        local_path = local_glob,
        sample_arg = sample_arg,
        seed = config.sampling_seed,
    );

    println!(
        "Sampling root table {} for ~{} rows ({:.5} percent of the input)...",
        root.name, target, percentage
    );
    let start_time = Instant::now();
    conn.execute_batch(&sql)
        .with_context(|| format!("Failed to sample root table '{}'", root.name))?;
    println!("Sampling took: {:?}", start_time.elapsed());

    println!("Removing root table data from local disk...");
    std::fs::remove_dir_all(&local_root_data_path)
        .with_context(|| format!("Failed to remove dir: '{}'", &local_root_data_path))?;

    // re-enable multi-threading
    conn.execute("RESET threads;", [])
        .with_context(|| "Failed to reset thread count to default")?;

    let sampled_root_count: u64 = conn
        .query_row(
            &format!("SELECT count(*) FROM sampled_{}", root.name),
            [],
            |row| row.get(0),
        )
        .context("Failed to count sampled root rows")?;
    println!("  {} sampled: {sampled_root_count} rows", root.name);

    // Sample child tables by joining against their sampled parent.
    for &idx in &order[1..] {
        let table = &config.tables[idx];
        let parent = table.parent.as_ref().unwrap();
        let parent_join_key = table.parent_join_col.as_ref().unwrap();
        let join_key = table.join_col.as_ref().unwrap();
        let glob = parquet_glob_pattern(input, &table.name);

        println!(
            "Sampling child table '{}' (parent: '{parent}')...",
            table.name
        );

        let sql = format!(
            "CREATE TABLE sampled_{name} AS \
             SELECT DISTINCT c.* \
             FROM read_parquet('{glob}') c \
             WHERE c.\"{jk}\" IN 
                (SELECT {pk} from sampled_{parent})",
            name = table.name,
            parent = parent,
            jk = join_key,
            pk = parent_join_key,
        );
        conn.execute_batch(&sql)
            .with_context(|| format!("Failed to sample child table '{}'", table.name))?;

        let child_count: u64 = conn
            .query_row(
                &format!("SELECT count(*) FROM sampled_{}", table.name),
                [],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to count sampled rows for '{}'", table.name))?;
        println!("  {} sampled: {child_count} rows", table.name);
    }

    if args.dry_run {
        println!("\nDry run complete. No files were written.");
        return Ok(());
    }

    // Write output.
    println!("\nWriting sampled parquet files...");
    for &idx in &order {
        let table = &config.tables[idx];
        println!("  Writing '{}'...", table.name);
        let sql = format!(
            "COPY sampled_{name} TO '{output}/{name}' (FORMAT PARQUET, PER_THREAD_OUTPUT true)",
            name = table.name,
            output = output,
        );
        conn.execute_batch(&sql)
            .with_context(|| format!("Failed to write sampled table '{}'", table.name))?;
        println!("  {}: done", table.name);
    }

    println!("\nSampling complete.");
    Ok(())
}
