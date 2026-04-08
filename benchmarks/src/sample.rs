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
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Instant;

use crate::duckdb_utils::open_duckdb_conn;

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

#[derive(Deserialize)]
pub struct SampleConfig {
    pub root_table: String,
    pub sampling_seed: u64,
    pub tables: Vec<TableConfig>,
}

#[derive(Deserialize)]
pub struct TableConfig {
    pub name: String,
    pub parent: Option<String>,
    pub parent_join_col: Option<String>,
    pub join_col: Option<String>,
}

/// Returns table names in topological order (root first, then children).
fn topological_order(config: &SampleConfig) -> Result<Vec<usize>> {
    let name_to_idx: HashMap<&str, usize> = config
        .tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    let mut order = Vec::with_capacity(config.tables.len());
    let mut processed: HashSet<&str> = HashSet::new();

    // Start with the root table.
    let root_idx = *name_to_idx
        .get(config.root_table.as_str())
        .with_context(|| {
            format!(
                "Root table '{}' not found in tables list",
                config.root_table
            )
        })?;
    if config.tables[root_idx].parent.is_some() {
        bail!(
            "Root table '{}' cannot have a parent table.",
            config.root_table
        )
    }
    order.push(root_idx);
    processed.insert(&config.root_table);

    // Iteratively add tables whose parent has been processed. Repeat until no progress is made
    let mut progress = true;
    while progress {
        progress = false;
        for (i, table) in config.tables.iter().enumerate() {
            if processed.contains(table.name.as_str()) {
                continue;
            }
            if let Some(parent) = &table.parent {
                if processed.contains(parent.as_str()) {
                    // Validate that child tables have the required keys.
                    if table.parent_join_col.is_none() || table.join_col.is_none() {
                        bail!(
                            "Table '{}' has a parent '{}' but is missing parent_join_col or join_col",
                            table.name,
                            parent
                        );
                    }
                    order.push(i);
                    processed.insert(&table.name);
                    progress = true;
                }
            } else if table.name != config.root_table {
                bail!(
                    "Table '{}' has no parent and is not the root table '{}'",
                    table.name,
                    config.root_table
                );
            }
        }
    }

    // Check for unprocessed tables (cycle or missing parent).
    for table in &config.tables {
        if !processed.contains(table.name.as_str()) {
            bail!(
                "Table '{}' could not be processed. Its parent '{}' is not in the config or there is a cycle.",
                table.name,
                table.parent.as_deref().unwrap_or("(none)")
            );
        }
    }

    Ok(order)
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

fn validate_table_has_parquet_files(
    conn: &Connection,
    glob: &str,
    table_name: &str,
) -> Result<bool> {
    let sql = format!("SELECT count(*) FROM glob('{glob}') LIMIT 1");
    let count: usize = conn
        .query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("Failed to check parquet files for table '{table_name}'"))?;
    Ok(count > 0)
}

pub fn run_sample(args: SampleArgs) -> Result<()> {
    let config_content = fs::read_to_string(&args.config)
        .with_context(|| format!("Failed to read config file '{}'", args.config))?;
    let config: SampleConfig =
        toml::from_str(&config_content).context("Failed to parse sample config TOML")?;

    let conn = open_duckdb_conn()?;

    let input = args.input.trim_end_matches('/');
    let output = args.output.trim_end_matches('/');

    // Validate that all tables have parquet files.
    println!("Validating input paths...");
    let mut missing_tables = Vec::new();
    for table in &config.tables {
        let glob = parquet_glob_pattern(input, &table.name);
        if validate_table_has_parquet_files(&conn, &glob, &table.name)? {
            println!("  {}: ok", table.name);
        } else {
            println!("  {}: no parquet files found at '{glob}'", table.name);
            missing_tables.push(table.name.clone());
        }
    }

    if !missing_tables.is_empty() {
        bail!(
            "No parquet files found for {} table(s): {}. Aborting.",
            missing_tables.len(),
            missing_tables.join(", ")
        );
    }

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

    let threads_used: u64 = conn
        .query_row("SELECT current_setting('threads')", [], |row| row.get(0))
        .with_context(|| "Failed to get current thread count")?;

    let percentage = (target as f64 / total_rows as f64) * 100.0;
    let sql = format!(
        "CREATE TABLE sampled_{name} AS \
         SELECT * FROM  read_parquet('{local_path}') \
         USING SAMPLE system({percentage:.3} PERCENT) REPEATABLE({seed})",
        name = root.name,
        local_path = local_glob,
        percentage = percentage,
        seed = config.sampling_seed,
    );
    println!(
        "Sampling root table {} for ~{} rows ({:.3} percent of the input) using {threads_used} thread(s). This may take a while...",
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
             SELECT c.* \
             FROM read_parquet('{glob}') c \
             INNER JOIN sampled_{parent} p ON c.\"{jk}\" = p.\"{pk}\"",
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
            "COPY sampled_{name} TO '{output}/{name}/' (FORMAT PARQUET, PER_THREAD_OUTPUT true)",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(name: &str, parent: Option<&str>) -> TableConfig {
        TableConfig {
            name: name.to_string(),
            parent: parent.map(|s| s.to_string()),
            parent_join_col: parent.map(|_| "parent_id".to_string()),
            join_col: parent.map(|_| "id".to_string()),
        }
    }

    fn make_config(root_table: &str, tables: Vec<TableConfig>) -> SampleConfig {
        SampleConfig {
            root_table: root_table.to_string(),
            sampling_seed: 42,
            tables,
        }
    }

    #[test]
    fn single_root_table() {
        let config = make_config("orders", vec![make_table("orders", None)]);
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn root_with_one_child() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn root_with_multiple_children() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
                make_table("payments", Some("orders")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order[0], 0); // root first
        let rest: HashSet<usize> = order[1..].iter().copied().collect();
        assert_eq!(rest, HashSet::from([1, 2]));
    }

    #[test]
    fn multi_level_hierarchy() {
        // orders -> line_items -> shipments
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("orders")),
                make_table("shipments", Some("line_items")),
            ],
        );
        let order = topological_order(&config).unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn tables_listed_in_reverse_dependency_order() {
        // Config lists child before parent — should still resolve correctly.
        let config = make_config(
            "orders",
            vec![
                make_table("shipments", Some("line_items")),
                make_table("line_items", Some("orders")),
                make_table("orders", None),
            ],
        );
        let order = topological_order(&config).unwrap();
        // Root (orders=idx2) must come first, then line_items(idx1), then shipments(idx0).
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn error_root_table_not_in_list() {
        let config = make_config("missing", vec![make_table("orders", None)]);
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_root_table_has_parent() {
        let config = make_config("orders", vec![make_table("orders", Some("something"))]);
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_non_root_without_parent() {
        let config = make_config(
            "orders",
            vec![make_table("orders", None), make_table("orphan", None)],
        );
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_missing_parent_reference() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                make_table("line_items", Some("nonexistent")),
            ],
        );
        assert!(topological_order(&config).is_err());
    }

    #[test]
    fn error_child_missing_join_cols() {
        let config = make_config(
            "orders",
            vec![
                make_table("orders", None),
                TableConfig {
                    name: "line_items".to_string(),
                    parent: Some("orders".to_string()),
                    parent_join_col: None,
                    join_col: None,
                },
            ],
        );
        assert!(topological_order(&config).is_err());
    }
}
