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

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use paradedb::median;
use sqlx::{Connection, PgConnection, Row};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

mod config;
mod convert;
mod sample;
mod utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run benchmarks against a ParadeDB instance.
    Benchmark {
        #[command(subcommand)]
        mode: BenchmarkMode,
    },
    /// Convert parquet datasets in S3 to CSV format using DuckDB.
    Convert(convert::ConvertArgs),
    /// Sample a CSV dataset to a target row count, preserving table relationships.
    Sample(sample::SampleArgs),
}

#[derive(Subcommand)]
enum BenchmarkMode {
    /// Run benchmarks with synthetically generated data.
    Generated(GeneratedArgs),
    /// Run benchmarks with a pre-existing dataset loaded from CSV files.
    Existing(ExistingArgs),
}

#[derive(Parser)]
struct CommonBenchmarkArgs {
    #[arg(long, value_parser = ["pg_search"], default_value = "pg_search")]
    r#type: String,

    /// Postgres URL.
    #[arg(long)]
    url: String,

    /// Dataset to use.
    #[arg(long, default_value = "single")]
    dataset: String,

    /// Whether to pre-warm the dataset using `pg_prewarm`.
    #[arg(long, default_value = "true")]
    prewarm: bool,

    /// Whether to run `VACUUM ANALYZE` before executing queries.
    #[arg(long, default_value = "true")]
    vacuum: bool,

    /// Skip data setup and index creation. Assumes tables and indexes already exist.
    #[arg(long, default_value_t = false)]
    skip_setup: bool,

    /// Number of runs to execute for each query.
    #[arg(long, default_value = "3")]
    runs: usize,

    /// Output format.
    #[arg(long, value_parser = ["md", "csv", "json"], default_value = "md")]
    output: String,

    /// Whether to fail on query errors. Set to false for backfills against older versions
    /// that may not support all query syntax.
    #[arg(long, default_value_t = true, num_args = 1)]
    fail_on_error: bool,

    /// Whether to clear the OS page cache and Postgres buffer cache before each query.
    #[arg(long, default_value_t = true, num_args = 1)]
    clear_caches: bool,
}

#[derive(Parser)]
struct GeneratedArgs {
    #[command(flatten)]
    common: CommonBenchmarkArgs,

    /// Number of rows to insert (in the largest generated table for the dataset).
    #[arg(long, default_value = "10000000")]
    rows: u32,
}

#[derive(Parser)]
struct ExistingArgs {
    #[command(flatten)]
    common: CommonBenchmarkArgs,

    /// Size label for the pre-sampled dataset (e.g. "10k", "100k", "1m").
    #[arg(long)]
    size: String,

    /// Base path to external CSV data source (S3 or local). Overrides s3_base_path in
    /// config.toml. CSVs are loaded from `{data_source}/sampled/{size}/csv/{table}/`.
    #[arg(long)]
    data_source: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Benchmark { mode } => run_benchmark(mode).await,
        Commands::Convert(args) => convert::run_convert(args),
        Commands::Sample(args) => sample::run_sample(args),
    }
}

async fn run_benchmark(mode: BenchmarkMode) -> anyhow::Result<()> {
    match mode {
        BenchmarkMode::Generated(args) => run_benchmark_generated(args).await,
        BenchmarkMode::Existing(args) => run_benchmark_existing(args).await,
    }
}

async fn run_benchmark_generated(args: GeneratedArgs) -> anyhow::Result<()> {
    let common = &args.common;
    if !common.skip_setup {
        generate_test_data(&common.url, &common.dataset, args.rows)?
    }
    let rows_display = args.rows.to_string();
    run_sql_benchmarks(common, &rows_display).await
}

async fn run_benchmark_existing(args: ExistingArgs) -> anyhow::Result<()> {
    let common = &args.common;
    if !common.skip_setup {
        load_external_data(
            &common.url,
            &common.dataset,
            &args.size,
            args.data_source.as_deref(),
        )?;
    }
    run_sql_benchmarks(common, &args.size).await
}

async fn run_sql_benchmarks(args: &CommonBenchmarkArgs, rows_display: &str) -> anyhow::Result<()> {
    match args.output.as_str() {
        "md" => generate_markdown_output(args, rows_display).await,
        "csv" => generate_csv_output(args, rows_display).await,
        "json" => generate_json_output(args, rows_display).await,
        _ => unreachable!("Clap ensures only md, csv, or json are valid"),
    }
}

struct IndexCreationResult {
    duration_min_ms: f64,
    index_name: String,
    index_size: i64,
    segment_count: i64,
}

struct QueryResult {
    query_type: String,
    query: String,
    runtimes_ms: Vec<f64>,
    num_results: usize,
}

#[derive(serde::Serialize)]
struct JSONBenchmarkResult {
    name: String,
    unit: &'static str,
    value: f64,
    extra: String,
}

impl From<QueryResult> for JSONBenchmarkResult {
    fn from(res: QueryResult) -> Self {
        let median = median(res.runtimes_ms.iter());
        let cold_query_extra = res
            .runtimes_ms
            .first()
            .map(|ms| format!("cold_query_ms={ms:.3}; query={}", res.query))
            .unwrap_or_else(|| format!("cold_query_ms=NA; query={}", res.query));

        Self {
            name: res.query_type,
            unit: "median ms",
            value: median,
            extra: cold_query_extra,
        }
    }
}

async fn process_index_creation(
    args: &CommonBenchmarkArgs,
) -> anyhow::Result<Vec<IndexCreationResult>> {
    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let index_sql = format!("datasets/{}/create_index/{}.sql", args.dataset, args.r#type);
    let mut results = Vec::new();

    for statement in queries(Path::new(&index_sql)) {
        println!("{statement}");

        let start = Instant::now();
        sqlx::query(&statement)
            .execute(&mut conn)
            .await
            .with_context(|| "Failed to execute index creation SQL")?;
        let duration_min_ms = start.elapsed().as_secs_f64() / 60.0;

        let index_name = extract_index_name(&statement).to_owned();

        let row = sqlx::query(&format!(
            "SELECT pg_relation_size('{index_name}') / (1024 * 1024)"
        ))
        .fetch_one(&mut conn)
        .await
        .with_context(|| "Failed to get index size")?;
        let index_size: i64 = row.get(0);

        let row = sqlx::query(&format!(
            "SELECT count(*) FROM paradedb.index_info('{index_name}')"
        ))
        .fetch_one(&mut conn)
        .await
        .with_context(|| "Failed to get segment count")?;
        let segment_count: i64 = row.get(0);

        results.push(IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        });
    }

    Ok(results)
}

async fn run_benchmarks(args: &CommonBenchmarkArgs) -> anyhow::Result<Vec<QueryResult>> {
    let mut utility_conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;

    if args.vacuum {
        sqlx::query("VACUUM ANALYZE")
            .execute(&mut utility_conn)
            .await
            .with_context(|| "Failed to vacuum")?;
    }

    if args.prewarm {
        prewarm_indexes(&mut utility_conn, &args.dataset, &args.r#type).await?;
    }

    if let Err(err) = ensure_pg_buffercache_extension(&mut utility_conn).await {
        eprintln!("WARNING: Failed to initialize pg_buffercache extension: {err}");
    }

    // Locate all query paths, and sort them for stability in the output.
    let queries_dir = format!("datasets/{}/queries/{}", args.dataset, args.r#type);
    let query_paths: anyhow::Result<Vec<Option<_>>> = std::fs::read_dir(queries_dir)
        .with_context(|| "Failed to read queries directory")?
        .map(|entry| {
            let entry = entry.with_context(|| "Failed to read directory entry")?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("sql") {
                // Not a query file.
                return Ok(None);
            }
            Ok(Some(path))
        })
        .collect();
    let mut query_paths: Vec<_> = query_paths?.into_iter().flatten().collect();
    query_paths.sort_unstable();

    let mut results = Vec::new();
    for path in query_paths {
        for (query_type, query) in benchmark_queries(&path) {
            if args.clear_caches {
                if let Err(err) = clear_caches(&mut utility_conn).await {
                    panic!("Failed to clear caches before query: {err}");
                }
            }
            println!("Query Type: {query_type}\nQuery: {query}");
            let result = execute_query_multiple_times(
                &args.url,
                &query_type,
                &query,
                args.runs,
                args.fail_on_error,
            )
            .await?;
            match result {
                Some((runtimes_ms, num_results)) => {
                    println!("Results: {runtimes_ms:?} | Rows Returned: {num_results}\n");
                    results.push(QueryResult {
                        query_type,
                        query,
                        runtimes_ms,
                        num_results,
                    });
                }
                None => {
                    println!("Skipped (query error)\n");
                }
            }
        }
    }

    Ok(results)
}

async fn generate_markdown_output(
    args: &CommonBenchmarkArgs,
    rows_display: &str,
) -> anyhow::Result<()> {
    let output_file = format!("results_{}.md", args.r#type);
    let mut file = File::create(&output_file).with_context(|| "Failed to create output file")?;

    write_benchmark_header(&mut file)?;
    write_test_info(&mut file, args, rows_display).await?;
    write_postgres_settings(&mut file, &args.url).await?;
    if !args.skip_setup {
        process_index_creation_md(&mut file, args).await?;
    }
    run_benchmarks_md(&mut file, args).await?;
    Ok(())
}

async fn generate_csv_output(args: &CommonBenchmarkArgs, rows_display: &str) -> anyhow::Result<()> {
    write_test_info_csv(args, rows_display).await?;
    write_postgres_settings_csv(&args.url, &args.r#type).await?;
    if !args.skip_setup {
        process_index_creation_csv(args).await?;
    }
    run_benchmarks_csv(args).await?;
    Ok(())
}

async fn generate_json_output(
    args: &CommonBenchmarkArgs,
    _rows_display: &str,
) -> anyhow::Result<()> {
    if !args.skip_setup {
        process_index_creation_json(args).await?;
    }
    run_benchmarks_json(args).await?;
    Ok(())
}

async fn write_test_info_csv(args: &CommonBenchmarkArgs, rows_display: &str) -> anyhow::Result<()> {
    let filename = format!("results_{}_test_info.csv", args.r#type);
    let mut file = File::create(&filename).with_context(|| "Failed to create test info CSV")?;

    writeln!(file, "Key,Value").unwrap();
    writeln!(file, "Dataset Size,{rows_display}")?;
    writeln!(file, "Test Type,{}", args.r#type)?;
    writeln!(file, "Prewarm,{}", args.prewarm)?;
    writeln!(file, "Vacuum,{}", args.vacuum)?;

    if args.r#type == "pg_search" {
        let mut conn = PgConnection::connect(&args.url)
            .await
            .with_context(|| "Failed to connect to database for version info")?;
        let row = sqlx::query("SELECT version, githash, build_mode FROM paradedb.version_info()")
            .fetch_one(&mut conn)
            .await
            .with_context(|| "Failed to fetch paradedb.version_info()")?;
        let version: String = row.get(0);
        let githash: String = row.get(1);
        let build_mode: String = row.get(2);
        writeln!(file, "pg_search Version,{version}")?;
        writeln!(file, "pg_search Git Hash,{githash}")?;
        writeln!(file, "pg_search Build Mode,{build_mode}")?;
    }
    Ok(())
}

async fn write_postgres_settings_csv(url: &str, test_type: &str) -> anyhow::Result<()> {
    let filename = format!("results_{test_type}_postgres_settings.csv");
    let mut file =
        File::create(&filename).with_context(|| "Failed to create postgres settings CSV")?;

    writeln!(file, "Setting,Value").unwrap();

    let settings = vec![
        "maintenance_work_mem",
        "shared_buffers",
        "max_parallel_workers",
        "max_worker_processes",
        "max_parallel_workers_per_gather",
        "max_parallel_maintenance_workers",
    ];

    let mut conn = PgConnection::connect(url)
        .await
        .with_context(|| "Failed to connect to database")?;
    for setting in settings {
        let row = sqlx::query(&format!("SHOW {setting}"))
            .fetch_one(&mut conn)
            .await
            .with_context(|| "Failed to get postgres setting")?;
        let value: String = row.get(0);
        writeln!(file, "{setting},{value}")?;
    }
    Ok(())
}

async fn process_index_creation_csv(args: &CommonBenchmarkArgs) -> anyhow::Result<()> {
    let filename = format!("results_{}_index_creation.csv", args.r#type);
    let mut file =
        File::create(&filename).with_context(|| "Failed to create index creation CSV")?;

    writeln!(
        file,
        "Index Name,Duration (min),Index Size (MB),Segment Count"
    )?;

    for result in process_index_creation(args).await? {
        let IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        } = result;
        writeln!(
            file,
            "{index_name},{duration_min_ms:.2},{index_size},{segment_count}"
        )?;
    }
    Ok(())
}

async fn run_benchmarks_csv(args: &CommonBenchmarkArgs) -> anyhow::Result<()> {
    let filename = format!("results_{}_benchmark_results.csv", args.r#type);
    let mut file =
        File::create(&filename).with_context(|| "Failed to create benchmark results CSV")?;

    // Write header
    let mut header = String::from("Query Type");
    for i in 1..=args.runs {
        header.push_str(&format!(",Run {i} (ms)"));
    }
    header.push_str(",Rows Returned,Query");
    writeln!(file, "{header}")?;

    for result in run_benchmarks(args).await? {
        let QueryResult {
            query_type,
            query,
            runtimes_ms,
            num_results,
        } = result;

        let mut result_line = query_type;
        for &runtime_ms in &runtimes_ms {
            result_line.push_str(&format!(",{runtime_ms:.0}"));
        }
        result_line.push_str(&format!(
            ",{},\"{}\"",
            num_results,
            query.replace("\"", "\"\"")
        ));
        writeln!(file, "{result_line}")?;
    }
    Ok(())
}

fn write_benchmark_header(file: &mut File) -> anyhow::Result<()> {
    Ok(writeln!(file, "# Benchmark Results")?)
}

async fn write_test_info(
    file: &mut File,
    args: &CommonBenchmarkArgs,
    rows_display: &str,
) -> anyhow::Result<()> {
    writeln!(file, "\n## Test Info")?;
    writeln!(file, "| Key         | Value       |")?;
    writeln!(file, "|-------------|-------------|")?;
    writeln!(file, "| Dataset Size | {rows_display} |")?;
    writeln!(file, "| Test Type   | {} |", args.r#type)?;
    writeln!(file, "| Prewarm     | {} |", args.prewarm)?;
    writeln!(file, "| Vacuum      | {} |", args.vacuum)?;

    if args.r#type == "pg_search" {
        let mut conn = PgConnection::connect(&args.url)
            .await
            .with_context(|| "Failed to connect to database for version info")?;
        let row = sqlx::query("SELECT version, githash, build_mode FROM paradedb.version_info()")
            .fetch_one(&mut conn)
            .await
            .with_context(|| "Failed to fetch paradedb.version_info()")?;
        let version: String = row.get(0);
        let githash: String = row.get(1);
        let build_mode: String = row.get(2);
        writeln!(file, "| pg_search Version | {version} |")?;
        writeln!(file, "| pg_search Git Hash | {githash} |")?;
        writeln!(file, "| pg_search Build Mode | {build_mode} |")?;
    }
    Ok(())
}

async fn write_postgres_settings(file: &mut File, url: &str) -> anyhow::Result<()> {
    writeln!(file, "\n## Postgres Settings")?;
    writeln!(file, "| Setting                        | Value |")?;
    writeln!(file, "|--------------------------------|-------|")?;

    let settings = vec![
        "maintenance_work_mem",
        "shared_buffers",
        "max_parallel_workers",
        "max_worker_processes",
        "max_parallel_workers_per_gather",
    ];

    let mut conn = PgConnection::connect(url)
        .await
        .with_context(|| "Failed to connect to database")?;
    for setting in settings {
        let row = sqlx::query(&format!("SHOW {setting}"))
            .fetch_one(&mut conn)
            .await
            .with_context(|| "Failed to get postgres setting")?;
        let value: String = row.get(0);
        writeln!(file, "| {setting} | {value} |")?;
    }
    Ok(())
}

fn generate_test_data(url: &str, dataset: &str, rows: u32) -> anyhow::Result<()> {
    let status = Command::new("psql")
        .arg(url)
        .arg("-v")
        .arg(format!("rows={rows}"))
        .arg("-f")
        .arg(format!("datasets/{dataset}/generate.sql"))
        .status()
        .with_context(|| "Failed to create table")?;

    if !status.success() {
        bail!("Failed to create table");
    }
    Ok(())
}

fn load_external_data(
    url: &str,
    dataset: &str,
    size_label: &str,
    data_source: Option<&str>,
) -> anyhow::Result<()> {
    // Read dataset config for table names and S3 path.
    let config_path = format!("datasets/{dataset}/config.toml");
    let (config, _) = config::load_dataset_config(&config_path)
        .with_context(|| format!("Failed to load config '{config_path}'"))?;

    // Determine CSV data source path.
    let base_path = match data_source {
        Some(path) => path,
        None => config.s3_base_path.as_deref().with_context(|| {
            format!(
                "Dataset '{dataset}' has no S3 base path. Provide --data-source or set \
                 s3_base_path in datasets/{dataset}/config.toml"
            )
        })?,
    };
    let source_path = format!(
        "{}/sampled/{}/csv",
        base_path.trim_end_matches('/'),
        size_label
    );
    println!("Data source: {source_path}");

    // Create tables via DDL.
    let create_tables_sql = format!("datasets/{dataset}/create_tables.sql");
    if !Path::new(&create_tables_sql).exists() {
        bail!(
            "Dataset '{dataset}' requires create_tables.sql but none found at {create_tables_sql}"
        );
    }
    let status = Command::new("psql")
        .arg(url)
        .arg("-f")
        .arg(&create_tables_sql)
        .status()
        .with_context(|| "Failed to execute create_tables.sql")?;
    if !status.success() {
        bail!("Failed to create tables from {create_tables_sql}");
    }

    // Download CSV data from source and load into PostgreSQL.
    let temp_dir = format!("/tmp/benchmark_data/{dataset}");
    if Path::new(&temp_dir).exists() {
        std::fs::remove_dir_all(&temp_dir)
            .with_context(|| format!("Failed to clean temp directory '{temp_dir}'"))?;
    }
    std::fs::create_dir_all(&temp_dir)
        .with_context(|| format!("Failed to create temp directory '{temp_dir}'"))?;

    let duckdb_conn =
        utils::open_duckdb_conn().with_context(|| "Failed to open DuckDB connection")?;

    for table_name in config.all_table_names() {
        let csv_source = format!("{source_path}/{table_name}");
        let table_temp_dir = format!("{temp_dir}/{table_name}");
        std::fs::create_dir_all(&table_temp_dir)
            .with_context(|| format!("Failed to create temp directory '{table_temp_dir}'"))?;

        // Download CSV files from source to local temp dir.
        // We must use parallel=false because some datasets (stackoverflow for instance) contain a
        // bunch of code or json that duckdbs parallel parser can't handle if one of its chunk
        // boundaries ends up in one of the complicated-quote/line-break blocks common to those
        // datasets
        println!("Downloading CSVs for '{table_name}' from {csv_source}...");
        let download_sql = format!(
            "COPY (SELECT * FROM read_csv('{csv_source}/*.csv', header=true, parallel=false)) \
             TO '{table_temp_dir}' (FORMAT CSV, HEADER true, PER_THREAD_OUTPUT true)"
        );
        duckdb_conn
            .execute_batch(&download_sql)
            .with_context(|| format!("Failed to download CSV for table '{table_name}'"))?;

        // Load each local CSV file into PostgreSQL.
        println!("Loading '{table_name}' into PostgreSQL...");
        let local_csvs: Vec<_> = std::fs::read_dir(&table_temp_dir)
            .with_context(|| format!("Failed to read temp directory '{table_temp_dir}'"))?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension().and_then(|s| s.to_str()) == Some("csv") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        for csv_path in &local_csvs {
            let csv_str = csv_path.to_string_lossy();
            let copy_cmd = format!("\\copy \"{table_name}\" FROM '{csv_str}' CSV HEADER");
            let status = Command::new("psql")
                .arg(url)
                .arg("-c")
                .arg(&copy_cmd)
                .status()
                .with_context(|| "Failed to execute psql copy")?;
            if !status.success() {
                bail!("Failed to load '{csv_str}' into table '{table_name}'");
            }
        }
        println!("  Loaded {} file(s) into '{table_name}'.", local_csvs.len());
    }

    // Cleanup temp files.
    println!("Cleaning up temp files...");
    if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
        eprintln!("Warning: Failed to clean up temp directory '{temp_dir}': {e}");
    }

    println!("External data loaded successfully.");
    Ok(())
}

async fn process_index_creation_md(
    file: &mut File,
    args: &CommonBenchmarkArgs,
) -> anyhow::Result<()> {
    writeln!(file, "\n## Index Creation Results")?;
    writeln!(
        file,
        "| Index Name | Duration (min) | Index Size (MB) | Segment Count |"
    )?;
    writeln!(
        file,
        "|------------|----------------|-----------------|---------------|"
    )?;

    for result in process_index_creation(args).await? {
        let IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        } = result;

        writeln!(
            file,
            "| {index_name} | {duration_min_ms:.2} | {index_size} | {segment_count} |"
        )?;
    }
    Ok(())
}

async fn run_benchmarks_md(file: &mut File, args: &CommonBenchmarkArgs) -> anyhow::Result<()> {
    writeln!(file, "\n## Benchmark Results")?;

    write_benchmark_table_header(file, args.runs)?;

    for result in run_benchmarks(args).await? {
        let QueryResult {
            query_type,
            query,
            runtimes_ms,
            num_results,
        } = result;
        let md_query = query.replace("|", "\\|");
        write_benchmark_results_md(file, &query_type, &runtimes_ms, num_results, &md_query)?;
    }
    Ok(())
}

fn write_benchmark_table_header(file: &mut File, runs: usize) -> anyhow::Result<()> {
    let mut header = String::from("| Query Type ");
    let mut separator = String::from("|------------");

    for i in 1..=runs {
        header.push_str(&format!("| Run {i} (ms) "));
        separator.push_str("|------------");
    }

    header.push_str("| Rows Returned | Query |");
    separator.push_str("|---------------|--------|");

    writeln!(file, "{header}")?;
    writeln!(file, "{separator}")?;
    Ok(())
}

fn write_benchmark_results_md(
    file: &mut File,
    query_type: &str,
    results: &[f64],
    num_results: usize,
    md_query: &str,
) -> anyhow::Result<()> {
    let mut result_line = format!("| {query_type} ");

    for &result in results {
        result_line.push_str(&format!("| {result:.0} "));
    }

    result_line.push_str(&format!("| {num_results} | `{md_query}` |"));
    writeln!(file, "{result_line}")?;
    Ok(())
}

async fn process_index_creation_json(args: &CommonBenchmarkArgs) -> anyhow::Result<()> {
    for _result in process_index_creation(args).await? {
        // TODO: Record index creation results as JSON.
    }
    Ok(())
}

async fn run_benchmarks_json(args: &CommonBenchmarkArgs) -> anyhow::Result<()> {
    let mut file = File::create("results.json").with_context(|| "Failed to create output file")?;
    let results = run_benchmarks(args)
        .await?
        .into_iter()
        .map(JSONBenchmarkResult::from)
        .collect::<Vec<_>>();
    let results_json =
        serde_json::to_string(&results).with_context(|| "Failed to serialize results")?;
    file.write_all(results_json.as_bytes())
        .with_context(|| "Failed to write results")?;
    Ok(())
}

///
/// Return a Vec of the query strings contained in the given file path.
///
/// Strips comments and flattens each query onto a single line.
///
/// Will only split on semicolons with trailing newlines, which allows for applying GUCs to queries.
///
fn queries(file: &Path) -> Vec<String> {
    let content = std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("Failed to read file `{file:?}`: {e}"));

    content
        .split(";\n")
        .filter_map(|query| {
            let query = query
                .trim()
                .split("\n")
                .map(|line| line.split("--").next().unwrap().trim())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_owned();
            if query.is_empty() {
                None
            } else {
                Some(query)
            }
        })
        .collect()
}

fn benchmark_queries(file: &Path) -> Vec<(String, String)> {
    let query_type = file
        .file_stem()
        .unwrap_or_else(|| panic!("Failed to get file stem for `{}`", file.display()))
        .to_string_lossy()
        .into_owned();

    queries(file)
        .into_iter()
        .enumerate()
        .map(|(idx, query)| {
            let query_type = if idx == 0 {
                query_type.clone()
            } else {
                format!("{query_type} - alternative {idx}")
            };
            (query_type, query)
        })
        .collect()
}

fn extract_index_name(statement: &str) -> &str {
    statement
        .split_whitespace()
        .nth(2)
        .expect("Failed to parse index name")
}

async fn prewarm_indexes(
    conn: &mut PgConnection,
    dataset: &str,
    r#type: &str,
) -> anyhow::Result<()> {
    let prewarm_sql = format!("datasets/{dataset}/prewarm/{type}.sql");
    for statement in queries(Path::new(&prewarm_sql)) {
        sqlx::query(&statement)
            .execute(&mut *conn)
            .await
            .with_context(|| "Failed to prewarm indexes")?;
    }
    Ok(())
}

/// Execute a benchmark query multiple times on a single reused connection.
///
/// This creates a fresh connection for each benchmark query and then reuses it across repeated
/// runs of that query.
///
/// Uses the simple query protocol (via `raw_sql`) to match `psql` behavior, which is
/// necessary for compatibility with custom scan providers. Compound statements
/// (e.g., `SET ...; SELECT ...`) are handled natively by the simple protocol.
///
/// Timing uses `Instant` around `execute()`, which consumes the entire result set from
/// the wire without per-row object allocation, matching how `psql` with `\timing` works.
///
/// Returns `None` when `fail_on_error` is false and the query errors (the query is skipped).
async fn execute_query_multiple_times(
    url: &str,
    query_type: &str,
    query: &str,
    times: usize,
    fail_on_error: bool,
) -> anyhow::Result<Option<(Vec<f64>, usize)>> {
    let mut conn = PgConnection::connect(url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let mut results = Vec::new();
    let mut num_results = 0;

    for i in 0..times {
        let start = Instant::now();
        let result = sqlx::raw_sql(query).execute(&mut conn).await;
        let elapsed = start.elapsed();

        match result {
            Ok(r) => {
                results.push(elapsed.as_secs_f64() * 1000.0);
                if i == 0 {
                    num_results = r.rows_affected() as usize;
                }
            }
            Err(err) => {
                if fail_on_error {
                    panic!("Failed to execute benchmark query `{query_type}`:  {err}");
                } else {
                    eprintln!("WARNING: Skipping query `{query_type}` due to error: {err}");
                    return Ok(None);
                }
            }
        }
    }

    Ok(Some((results, num_results)))
}

fn drop_os_page_cache() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("sh")
            .arg("-c")
            // Use non-interactive sudo so local runs don't hang on a password prompt.
            .arg("sync; echo 3 | sudo -n tee /proc/sys/vm/drop_caches > /dev/null")
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let details = if !stderr.is_empty() { stderr } else { stdout };
        Err(format!(
            "linux cache-drop command failed (`sync; echo 3 | sudo -n tee /proc/sys/vm/drop_caches > /dev/null`): {details}"
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        // No portable equivalent in this benchmark runner today.
        Err("unsupported platform (cache-drop is only implemented on Linux; pass --clear-caches=false to disable)".to_string())
    }
}

async fn clear_caches(conn: &mut PgConnection) -> Result<(), String> {
    let mut errors = Vec::new();

    if let Err(err) = drop_os_page_cache() {
        errors.push(format!("OS page cache: {err}"));
    }
    if let Err(err) = evict_postgres_buffer_cache(conn).await {
        errors.push(format!("PostgreSQL buffer cache: {err}"));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join(" | "))
    }
}

async fn ensure_pg_buffercache_extension(conn: &mut PgConnection) -> anyhow::Result<()> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_buffercache")
        .execute(&mut *conn)
        .await
        .with_context(|| {
            "failed to create pg_buffercache extension (`CREATE EXTENSION IF NOT EXISTS pg_buffercache`"
        })?;
    Ok(())
}

async fn evict_postgres_buffer_cache(conn: &mut PgConnection) -> Result<(), String> {
    let sql = "DO $$ \
               BEGIN \
                   PERFORM pg_buffercache_evict(bufferid) \
                   FROM pg_buffercache \
                   WHERE relfilenode IS NOT NULL; \
               END \
               $$";
    sqlx::query(sql).execute(&mut *conn).await.map_err(|e| {
        format!("failed to evict PostgreSQL buffer cache via pg_buffercache (`{sql}`): {e}")
    })?;
    Ok(())
}
