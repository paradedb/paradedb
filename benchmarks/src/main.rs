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
use paradedb::{confidence_interval_half_width, mean, Window};
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
    Benchmark(BenchmarkArgs),
    /// Convert parquet datasets in S3 to CSV format using DuckDB.
    Convert(convert::ConvertArgs),
    /// Sample a CSV dataset to a target row count, preserving table relationships.
    Sample(sample::SampleArgs),
}

#[derive(Parser)]
struct BenchmarkArgs {
    /// Postgres URL.
    #[arg(long)]
    url: String,

    /// Dataset to use.
    #[arg(long, default_value = "stackoverflow")]
    dataset: String,

    /// Whether to pre-warm the dataset using `pg_prewarm`.
    #[arg(long, default_value_t = true, num_args = 1)]
    prewarm: bool,

    /// Whether to run `VACUUM ANALYZE` before executing queries.
    #[arg(long, default_value_t = true, num_args = 1)]
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
        Commands::Benchmark(args) => run_benchmark(args).await,
        Commands::Convert(args) => convert::run_convert(args),
        Commands::Sample(args) => sample::run_sample(args),
    }
}

async fn run_benchmark(args: BenchmarkArgs) -> anyhow::Result<()> {
    if !args.skip_setup {
        load_external_data(
            &args.url,
            &args.dataset,
            &args.size,
            args.data_source.as_deref(),
        )?;
    }
    run_sql_benchmarks(&args).await
}

async fn run_sql_benchmarks(args: &BenchmarkArgs) -> anyhow::Result<()> {
    match args.output.as_str() {
        "md" => generate_markdown_output(args).await,
        "csv" => generate_csv_output(args).await,
        "json" => generate_json_output(args).await,
        _ => unreachable!("Clap ensures only md, csv, or json are valid"),
    }
}

#[derive(Default)]
pub struct QueryRunResults {
    pub cold: f64,
    pub samples: Vec<f64>,
    pub num_results: usize,
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
    results: QueryRunResults,
}

#[derive(serde::Serialize)]
struct JSONBenchmarkResult {
    name: String,
    unit: &'static str,
    value: f64,
    range: String,
    extra: String,
    // Carried into the JSON output so a post-process step can verify
    // alternative-vs-base equality (MPP vs non-MPP).
    num_results: usize,
}

impl From<QueryResult> for JSONBenchmarkResult {
    fn from(res: QueryResult) -> Self {
        let mean = mean(&res.results.samples);
        let ci_half_width = confidence_interval_half_width(&res.results.samples, 0.95);

        let cold_query_extra =
            format!("cold_query_ms={:.3}; query={}", res.results.cold, res.query);
        let range_str = format!("±{ci_half_width:.3} ms");

        println!(
            r"Query results: |
            query: {},
            mean: {mean:.3} ms,
            confidence interval: ±{ci_half_width:.3} ms",
            res.query
        );

        Self {
            name: res.query_type,
            unit: "mean ms",
            value: mean,
            range: range_str,
            extra: cold_query_extra,
            num_results: res.results.num_results,
        }
    }
}

async fn process_index_creation(args: &BenchmarkArgs) -> anyhow::Result<Vec<IndexCreationResult>> {
    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let index_sql = format!("datasets/{}/create_index.sql", args.dataset);
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

async fn process_after_create_index_sql(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let after_create_index_sql = format!("datasets/{}/after_create_index.sql", args.dataset);
    if Path::new(&after_create_index_sql).exists() {
        let status = Command::new("psql")
            .arg(&args.url)
            .arg("-f")
            .arg(&after_create_index_sql)
            .status()
            .with_context(|| "Failed to execute after_create_index.sql")?;
        if !status.success() {
            bail!("Failed to create tables from {after_create_index_sql}");
        }
    }
    Ok(())
}

async fn run_benchmarks(args: &BenchmarkArgs) -> anyhow::Result<Vec<QueryResult>> {
    let mut utility_conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;

    println!("vacuuming...");
    if args.vacuum {
        sqlx::query("VACUUM FULL ANALYZE")
            .execute(&mut utility_conn)
            .await
            .with_context(|| "Failed to vacuum")?;
    }

    if args.prewarm {
        prewarm_indexes(&mut utility_conn, &args.dataset).await?;
    }

    if let Err(err) = ensure_pg_buffercache_extension(&mut utility_conn).await {
        eprintln!("WARNING: Failed to initialize pg_buffercache extension: {err}");
    }

    // Locate all query paths, and sort them for stability in the output.
    let queries_dir = format!("datasets/{}/queries", args.dataset);
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

            sqlx::raw_sql("CHECKPOINT;")
                .execute(&mut utility_conn)
                .await
                .with_context(|| "Failed to execute checkpoint.")?;

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
                Some(query_results) => {
                    println!(
                        "Results: [cold: {:?} ] {:?} | Rows Returned: {}\n",
                        query_results.cold, query_results.samples, query_results.num_results
                    );
                    results.push(QueryResult {
                        query_type,
                        query,
                        results: query_results,
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

async fn generate_markdown_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let output_file = "results_pg_search.md";
    let mut file = File::create(output_file).with_context(|| "Failed to create output file")?;

    write_benchmark_header(&mut file)?;
    write_test_info(&mut file, args).await?;
    write_postgres_settings(&mut file, &args.url).await?;
    if !args.skip_setup {
        process_index_creation_md(&mut file, args).await?;
        process_after_create_index_sql(args).await?;
    }
    run_benchmarks_md(&mut file, args).await?;
    Ok(())
}

async fn generate_csv_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    write_test_info_csv(args).await?;
    write_postgres_settings_csv(&args.url).await?;
    if !args.skip_setup {
        process_index_creation_csv(args).await?;
        process_after_create_index_sql(args).await?;
    }
    run_benchmarks_csv(args).await?;
    Ok(())
}

async fn generate_json_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    if !args.skip_setup {
        process_index_creation_json(args).await?;
        process_after_create_index_sql(args).await?;
    }
    run_benchmarks_json(args).await?;
    Ok(())
}

async fn write_test_info_csv(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let filename = "results_pg_search_test_info.csv";
    let mut file = File::create(filename).with_context(|| "Failed to create test info CSV")?;

    writeln!(file, "Key,Value").unwrap();
    writeln!(file, "Dataset Size,{}", args.size)?;
    writeln!(file, "Prewarm,{}", args.prewarm)?;
    writeln!(file, "Vacuum,{}", args.vacuum)?;

    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database for version info")?;
    let row = sqlx::query("SELECT version, build_mode FROM paradedb.version_info()")
        .fetch_one(&mut conn)
        .await
        .with_context(|| "Failed to fetch paradedb.version_info()")?;
    let version: String = row.get(0);
    let build_mode: String = row.get(1);
    writeln!(file, "pg_search Version,{version}")?;
    writeln!(file, "pg_search Build Mode,{build_mode}")?;
    Ok(())
}

async fn write_postgres_settings_csv(url: &str) -> anyhow::Result<()> {
    let filename = "results_pg_search_postgres_settings.csv";
    let mut file =
        File::create(filename).with_context(|| "Failed to create postgres settings CSV")?;

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

async fn process_index_creation_csv(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let filename = "results_pg_search_index_creation.csv";
    let mut file = File::create(filename).with_context(|| "Failed to create index creation CSV")?;

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

async fn run_benchmarks_csv(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let filename = "results_{}_benchmark_results.csv";
    let mut file =
        File::create(filename).with_context(|| "Failed to create benchmark results CSV")?;

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
            results,
        } = result;

        let mut result_line = query_type;
        for &runtime_ms in &results.samples {
            result_line.push_str(&format!(",{runtime_ms:.0}"));
        }
        result_line.push_str(&format!(
            ",{},\"{}\"",
            results.num_results,
            query.replace("\"", "\"\"")
        ));
        writeln!(file, "{result_line}")?;
    }
    Ok(())
}

fn write_benchmark_header(file: &mut File) -> anyhow::Result<()> {
    Ok(writeln!(file, "# Benchmark Results")?)
}

async fn write_test_info(file: &mut File, args: &BenchmarkArgs) -> anyhow::Result<()> {
    writeln!(file, "\n## Test Info")?;
    writeln!(file, "| Key         | Value       |")?;
    writeln!(file, "|-------------|-------------|")?;
    writeln!(file, "| Dataset Size | {} |", args.size)?;
    writeln!(file, "| Prewarm     | {} |", args.prewarm)?;
    writeln!(file, "| Vacuum      | {} |", args.vacuum)?;

    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database for version info")?;
    let row = sqlx::query("SELECT version, build_mode FROM paradedb.version_info()")
        .fetch_one(&mut conn)
        .await
        .with_context(|| "Failed to fetch paradedb.version_info()")?;
    let version: String = row.get(0);
    let build_mode: String = row.get(1);
    writeln!(file, "| pg_search Version | {version} |")?;
    writeln!(file, "| pg_search Build Mode | {build_mode} |")?;
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

async fn process_index_creation_md(file: &mut File, args: &BenchmarkArgs) -> anyhow::Result<()> {
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

async fn run_benchmarks_md(file: &mut File, args: &BenchmarkArgs) -> anyhow::Result<()> {
    writeln!(file, "\n## Benchmark Results")?;

    write_benchmark_table_header(file, args.runs)?;

    for result in run_benchmarks(args).await? {
        let QueryResult {
            query_type,
            query,
            results,
        } = result;
        let md_query = query.replace("|", "\\|");
        write_benchmark_results_md(
            file,
            &query_type,
            &results.samples,
            results.num_results,
            &md_query,
        )?;
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

async fn process_index_creation_json(args: &BenchmarkArgs) -> anyhow::Result<()> {
    for _result in process_index_creation(args).await? {
        // TODO: Record index creation results as JSON.
    }
    Ok(())
}

async fn run_benchmarks_json(args: &BenchmarkArgs) -> anyhow::Result<()> {
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

async fn prewarm_indexes(conn: &mut PgConnection, dataset: &str) -> anyhow::Result<()> {
    let prewarm_sql = format!("datasets/{dataset}/prewarm.sql");
    for statement in queries(Path::new(&prewarm_sql)) {
        sqlx::query(&statement)
            .execute(&mut *conn)
            .await
            .with_context(|| "Failed to prewarm indexes")?;
    }
    Ok(())
}

async fn get_query_id(query: &str, conn: &mut PgConnection) -> anyhow::Result<i64> {
    let explain_str = format!("EXPLAIN (VERBOSE, FORMAT JSON) {query}");
    let sqlx::types::Json(res): sqlx::types::Json<serde_json::Value> =
        sqlx::query_scalar(&explain_str).fetch_one(conn).await?;

    let query_id = res[0]["Query Identifier"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Failed to find query id"))?;

    Ok(query_id)
}

/// Execute a benchmark query, taking sample_count warmed samples on a single reused connection.
///
/// This creates a fresh connection for each benchmark query and then reuses it across repeated
/// runs of that query.
///
/// The query will be ran repeatedly, warming it,until a 3-run window shows a sub-0.1% ratio of
/// variance over mean, or it has been ran 10 times. At that point, sample_count samples will
/// be taken.
///
/// Uses the simple query protocol (via `raw_sql`) to match `psql` behavior, which is
/// necessary for compatibility with custom scan providers. Compound statements
/// (e.g., `SET ...; SELECT ...`) are handled natively by the simple protocol.
///
/// Timing uses the results of server-side planning + execution time from pg_stat_statements,
/// limiting the amount of non-extension-code time captured.
///
/// Returns `None` when `fail_on_error` is false and the query errors (the query is skipped).
async fn execute_query_multiple_times(
    url: &str,
    query_type: &str,
    query: &str,
    sample_count: usize,
    fail_on_error: bool,
) -> anyhow::Result<Option<QueryRunResults>> {
    let mut conn = PgConnection::connect(url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let mut window = Window::new(3);
    let mut results = QueryRunResults::default();

    let measured_query = query.split(";").last().unwrap().trim();
    // SELECT the times for the last query run, making sure we don't accidentally get the 'reset'
    // query
    let stats_reset_query = "SELECT pg_stat_statements_reset();";

    let query_id = get_query_id(measured_query, &mut conn).await?;
    let stats_query = format!("SELECT max_exec_time, max_plan_time, rows FROM pg_stat_statements WHERE queryid = {query_id};");

    // run until run-to-run variance is sub-0.1% (query is warmed) or
    // until 10 runs have passed, then take the next sample_count results
    let mut runs_completed = 0;
    let mut samples_taken = 0;
    while samples_taken < sample_count {
        let result: anyhow::Result<(f64, f64, i64)> = {
            sqlx::raw_sql(stats_reset_query)
                .execute(&mut conn)
                .await
                .with_context(|| format!("Failed to execute query: {stats_reset_query}"))?;
            sqlx::raw_sql(query)
                .execute(&mut conn)
                .await
                .with_context(|| format!("Failed to execute query: {query}"))?;
            let res = sqlx::query_as(&stats_query)
                .fetch_one(&mut conn)
                .await
                .with_context(|| format!("Failed to execute query: {stats_query}"))?;
            Ok(res)
        };

        match result {
            Ok((exec_time_ms, plan_time_ms, rows)) => {
                let time = exec_time_ms + plan_time_ms;
                window.push(time);
                if runs_completed == 0 {
                    results.num_results = rows as usize;
                    results.cold = time;
                } else if (window.is_full()
                    && window
                        .variance_over_mean()
                        .filter(|v| *v <= 0.001)
                        .is_some())
                    || runs_completed >= 10
                {
                    // only record once the query is sufficiently warm, or if we've already ran 10
                    results.samples.push(time);
                    samples_taken += 1;
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

        runs_completed += 1;
    }

    Ok(Some(results))
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

async fn evict_postgres_buffer_cache(conn: &mut PgConnection) -> anyhow::Result<()> {
    let evict_query = "SELECT pg_buffercache_evict_all();";
    sqlx::raw_sql(evict_query)
        .execute(conn)
        .await
        .with_context(|| format!("Failed to PostgreSQL buffer cache: {evict_query}"))?;
    Ok(())
}
