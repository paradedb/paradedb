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

use clap::Parser;
use paradedb::median;
use paradedb::micro_benchmarks::benchmark_columnar;
use sqlx::{Connection, PgConnection, Row};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
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

    /// True to skip creating the dataset, and only run queries.
    #[arg(long, default_value = "false")]
    existing: bool,

    /// Number of rows to insert (in the largest generated table for the dataset).
    #[arg(long, default_value = "10000000")]
    rows: u32,

    /// Number of runs to execute for each query.
    #[arg(long, default_value = "3")]
    runs: usize,

    /// Output format.
    #[arg(long, value_parser = ["md", "csv", "json"], default_value = "md")]
    output: String,

    #[arg(long, value_parser = ["fastfields", "sql"], default_value = "sql")]
    benchmark: String,

    #[arg(long, default_value = "2")]
    warmups: usize,

    #[arg(long, default_value = "5")]
    iterations: usize,

    #[arg(long, default_value = "100000")]
    batch_size: usize,

    /// Whether to fail on query errors. Set to false for backfills against older versions
    /// that may not support all query syntax.
    #[arg(long, default_value_t = true, num_args = 1)]
    fail_on_error: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.benchmark == "fastfields" {
        let mut conn = PgConnection::connect(&args.url).await.unwrap();
        let res = benchmark_columnar(
            &mut conn,
            args.existing,
            args.runs,
            args.warmups,
            args.rows as usize,
            args.batch_size,
        )
        .await;
        println!("Columnar Benchmark Completed: {res:?}");
    } else if args.benchmark == "sql" {
        if !args.existing {
            generate_test_data(&args.url, &args.dataset, args.rows);
        }

        match args.output.as_str() {
            "md" => generate_markdown_output(&args).await,
            "csv" => generate_csv_output(&args).await,
            "json" => generate_json_output(&args).await,
            _ => unreachable!("Clap ensures only md, csv, or json are valid"),
        }
    } else {
        eprintln!("Invalid benchmark type");
        std::process::exit(1);
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

async fn process_index_creation(args: &Args) -> Vec<IndexCreationResult> {
    let mut conn = PgConnection::connect(&args.url)
        .await
        .expect("Failed to connect to database");
    let index_sql = format!("datasets/{}/create_index/{}.sql", args.dataset, args.r#type);
    let mut results = Vec::new();

    for statement in queries(Path::new(&index_sql)) {
        println!("{statement}");

        let start = Instant::now();
        sqlx::query(&statement)
            .execute(&mut conn)
            .await
            .expect("Failed to execute index creation SQL");
        let duration_min_ms = start.elapsed().as_secs_f64() / 60.0;

        let index_name = extract_index_name(&statement).to_owned();

        let row = sqlx::query(&format!(
            "SELECT pg_relation_size('{index_name}') / (1024 * 1024)"
        ))
        .fetch_one(&mut conn)
        .await
        .expect("Failed to get index size");
        let index_size: i64 = row.get(0);

        let row = sqlx::query(&format!(
            "SELECT count(*) FROM paradedb.index_info('{index_name}')"
        ))
        .fetch_one(&mut conn)
        .await
        .expect("Failed to get segment count");
        let segment_count: i64 = row.get(0);

        results.push(IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        });
    }

    results
}

async fn run_benchmarks(args: &Args) -> Vec<QueryResult> {
    let mut utility_conn = PgConnection::connect(&args.url)
        .await
        .expect("Failed to connect to database");

    if args.vacuum {
        sqlx::query("VACUUM ANALYZE")
            .execute(&mut utility_conn)
            .await
            .expect("Failed to vacuum");
    }

    if args.prewarm {
        prewarm_indexes(&mut utility_conn, &args.dataset, &args.r#type).await;
    }

    if let Err(err) = ensure_pg_buffercache_extension(&mut utility_conn).await {
        eprintln!("WARNING: Failed to initialize pg_buffercache extension: {err}");
    }

    // Locate all query paths, and sort them for stability in the output.
    let queries_dir = format!("datasets/{}/queries/{}", args.dataset, args.r#type);
    let mut query_paths = std::fs::read_dir(queries_dir)
        .expect("Failed to read queries directory")
        .flat_map(|entry| {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("sql") {
                // Not a query file.
                return None;
            }
            Some(path)
        })
        .collect::<Vec<_>>();
    query_paths.sort_by_key(|path| benchmark_query_sort_key(path));

    let mut results = Vec::new();
    for path in query_paths {
        let query_type = benchmark_query_type(&path);
        let query = benchmark_query(&path);
        let mut conn = PgConnection::connect(&args.url)
            .await
            .expect("Failed to connect to database");

        if let Err(err) = clear_caches(&mut utility_conn).await {
            panic!("Failed to clear caches before query: {err}");
        }
        println!("Query Type: {query_type}\nQuery: {query}");
        let result = execute_query_multiple_times(
            &mut conn,
            &query_type,
            &query,
            args.runs,
            args.fail_on_error,
        )
        .await;
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

    results
}

async fn generate_markdown_output(args: &Args) {
    let output_file = format!("results_{}.md", args.r#type);
    let mut file = File::create(&output_file).expect("Failed to create output file");

    write_benchmark_header(&mut file);
    write_test_info(&mut file, args).await;
    write_postgres_settings(&mut file, &args.url).await;
    if !args.existing {
        process_index_creation_md(&mut file, args).await;
    }
    run_benchmarks_md(&mut file, args).await;
}

async fn generate_csv_output(args: &Args) {
    write_test_info_csv(args).await;
    write_postgres_settings_csv(&args.url, &args.r#type).await;
    if !args.existing {
        process_index_creation_csv(args).await;
    }
    run_benchmarks_csv(args).await;
}

async fn generate_json_output(args: &Args) {
    if !args.existing {
        process_index_creation_json(args).await;
    }
    run_benchmarks_json(args).await;
}

async fn write_test_info_csv(args: &Args) {
    let filename = format!("results_{}_test_info.csv", args.r#type);
    let mut file = File::create(&filename).expect("Failed to create test info CSV");

    writeln!(file, "Key,Value").unwrap();
    writeln!(file, "Number of Rows,{}", args.rows).unwrap();
    writeln!(file, "Test Type,{}", args.r#type).unwrap();
    writeln!(file, "Prewarm,{}", args.prewarm).unwrap();
    writeln!(file, "Vacuum,{}", args.vacuum).unwrap();

    if args.r#type == "pg_search" {
        let mut conn = PgConnection::connect(&args.url)
            .await
            .expect("Failed to connect to database for version info");
        let row = sqlx::query("SELECT version, githash, build_mode FROM paradedb.version_info()")
            .fetch_one(&mut conn)
            .await
            .expect("Failed to fetch paradedb.version_info()");
        let version: String = row.get(0);
        let githash: String = row.get(1);
        let build_mode: String = row.get(2);
        writeln!(file, "pg_search Version,{version}").unwrap();
        writeln!(file, "pg_search Git Hash,{githash}").unwrap();
        writeln!(file, "pg_search Build Mode,{build_mode}").unwrap();
    }
}

async fn write_postgres_settings_csv(url: &str, test_type: &str) {
    let filename = format!("results_{test_type}_postgres_settings.csv");
    let mut file = File::create(&filename).expect("Failed to create postgres settings CSV");

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
        .expect("Failed to connect to database");
    for setting in settings {
        let row = sqlx::query(&format!("SHOW {setting}"))
            .fetch_one(&mut conn)
            .await
            .expect("Failed to get postgres setting");
        let value: String = row.get(0);
        writeln!(file, "{setting},{value}").unwrap();
    }
}

async fn process_index_creation_csv(args: &Args) {
    let filename = format!("results_{}_index_creation.csv", args.r#type);
    let mut file = File::create(&filename).expect("Failed to create index creation CSV");

    writeln!(
        file,
        "Index Name,Duration (min),Index Size (MB),Segment Count"
    )
    .unwrap();

    for result in process_index_creation(args).await {
        let IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        } = result;
        writeln!(
            file,
            "{index_name},{duration_min_ms:.2},{index_size},{segment_count}"
        )
        .unwrap();
    }
}

async fn run_benchmarks_csv(args: &Args) {
    let filename = format!("results_{}_benchmark_results.csv", args.r#type);
    let mut file = File::create(&filename).expect("Failed to create benchmark results CSV");

    // Write header
    let mut header = String::from("Query Type");
    for i in 1..=args.runs {
        header.push_str(&format!(",Run {i} (ms)"));
    }
    header.push_str(",Rows Returned,Query");
    writeln!(file, "{header}").unwrap();

    for result in run_benchmarks(args).await {
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
        writeln!(file, "{result_line}").unwrap();
    }
}

fn write_benchmark_header(file: &mut File) {
    writeln!(file, "# Benchmark Results").unwrap();
}

async fn write_test_info(file: &mut File, args: &Args) {
    writeln!(file, "\n## Test Info").unwrap();
    writeln!(file, "| Key         | Value       |").unwrap();
    writeln!(file, "|-------------|-------------|").unwrap();
    writeln!(file, "| Number of Rows | {} |", args.rows).unwrap();
    writeln!(file, "| Test Type   | {} |", args.r#type).unwrap();
    writeln!(file, "| Prewarm     | {} |", args.prewarm).unwrap();
    writeln!(file, "| Vacuum      | {} |", args.vacuum).unwrap();

    if args.r#type == "pg_search" {
        let mut conn = PgConnection::connect(&args.url)
            .await
            .expect("Failed to connect to database for version info");
        let row = sqlx::query("SELECT version, githash, build_mode FROM paradedb.version_info()")
            .fetch_one(&mut conn)
            .await
            .expect("Failed to fetch paradedb.version_info()");
        let version: String = row.get(0);
        let githash: String = row.get(1);
        let build_mode: String = row.get(2);
        writeln!(file, "| pg_search Version | {version} |").unwrap();
        writeln!(file, "| pg_search Git Hash | {githash} |").unwrap();
        writeln!(file, "| pg_search Build Mode | {build_mode} |").unwrap();
    }
}

async fn write_postgres_settings(file: &mut File, url: &str) {
    writeln!(file, "\n## Postgres Settings").unwrap();
    writeln!(file, "| Setting                        | Value |").unwrap();
    writeln!(file, "|--------------------------------|-------|").unwrap();

    let settings = vec![
        "maintenance_work_mem",
        "shared_buffers",
        "max_parallel_workers",
        "max_worker_processes",
        "max_parallel_workers_per_gather",
    ];

    let mut conn = PgConnection::connect(url)
        .await
        .expect("Failed to connect to database");
    for setting in settings {
        let row = sqlx::query(&format!("SHOW {setting}"))
            .fetch_one(&mut conn)
            .await
            .expect("Failed to get postgres setting");
        let value: String = row.get(0);
        writeln!(file, "| {setting} | {value} |").unwrap();
    }
}

fn generate_test_data(url: &str, dataset: &str, rows: u32) {
    let status = Command::new("psql")
        .arg(url)
        .arg("-v")
        .arg(format!("rows={rows}"))
        .arg("-f")
        .arg(format!("datasets/{dataset}/generate.sql"))
        .status()
        .expect("Failed to create table");

    if !status.success() {
        eprintln!("Failed to create table");
        std::process::exit(1);
    }
}

async fn process_index_creation_md(file: &mut File, args: &Args) {
    writeln!(file, "\n## Index Creation Results").unwrap();
    writeln!(
        file,
        "| Index Name | Duration (min) | Index Size (MB) | Segment Count |"
    )
    .unwrap();
    writeln!(
        file,
        "|------------|----------------|-----------------|---------------|"
    )
    .unwrap();

    for result in process_index_creation(args).await {
        let IndexCreationResult {
            duration_min_ms,
            index_name,
            index_size,
            segment_count,
        } = result;

        writeln!(
            file,
            "| {index_name} | {duration_min_ms:.2} | {index_size} | {segment_count} |"
        )
        .unwrap();
    }
}

async fn run_benchmarks_md(file: &mut File, args: &Args) {
    writeln!(file, "\n## Benchmark Results").unwrap();

    write_benchmark_table_header(file, args.runs);

    for result in run_benchmarks(args).await {
        let QueryResult {
            query_type,
            query,
            runtimes_ms,
            num_results,
        } = result;
        let md_query = query.replace("|", "\\|");
        write_benchmark_results_md(file, &query_type, &runtimes_ms, num_results, &md_query);
    }
}

fn write_benchmark_table_header(file: &mut File, runs: usize) {
    let mut header = String::from("| Query Type ");
    let mut separator = String::from("|------------");

    for i in 1..=runs {
        header.push_str(&format!("| Run {i} (ms) "));
        separator.push_str("|------------");
    }

    header.push_str("| Rows Returned | Query |");
    separator.push_str("|---------------|--------|");

    writeln!(file, "{header}").unwrap();
    writeln!(file, "{separator}").unwrap();
}

fn write_benchmark_results_md(
    file: &mut File,
    query_type: &str,
    results: &[f64],
    num_results: usize,
    md_query: &str,
) {
    let mut result_line = format!("| {query_type} ");

    for &result in results {
        result_line.push_str(&format!("| {result:.0} "));
    }

    result_line.push_str(&format!("| {num_results} | `{md_query}` |"));
    writeln!(file, "{result_line}").unwrap();
}

async fn process_index_creation_json(args: &Args) {
    for _result in process_index_creation(args).await {
        // TODO: Record index creation results as JSON.
    }
}

async fn run_benchmarks_json(args: &Args) {
    let mut file = File::create("results.json").expect("Failed to create output file");
    let results = run_benchmarks(args)
        .await
        .into_iter()
        .map(JSONBenchmarkResult::from)
        .collect::<Vec<_>>();
    let results_json = serde_json::to_string(&results).expect("Failed to serialize results");
    file.write_all(results_json.as_bytes())
        .expect("Failed to write results");
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

fn benchmark_query(file: &Path) -> String {
    let queries = queries(file);
    match queries.as_slice() {
        [query] => query.clone(),
        _ => panic!(
            "Expected exactly one benchmark query in `{}`; split multi-query files before benchmarking",
            file.display()
        ),
    }
}

fn benchmark_query_sort_key(file: &Path) -> (String, usize) {
    let stem = file
        .file_stem()
        .unwrap_or_else(|| panic!("Failed to get file stem for `{}`", file.display()))
        .to_string_lossy();

    match stem.rsplit_once("-alternative-") {
        Some((base, suffix)) => match suffix.parse::<usize>() {
            Ok(idx) => (base.to_string(), idx),
            Err(_) => (stem.into_owned(), 0),
        },
        None => (stem.into_owned(), 0),
    }
}

fn benchmark_query_type(file: &Path) -> String {
    let (base, idx) = benchmark_query_sort_key(file);
    if idx == 0 {
        base
    } else {
        format!("{base} - alternative {idx}")
    }
}

fn extract_index_name(statement: &str) -> &str {
    statement
        .split_whitespace()
        .nth(2)
        .expect("Failed to parse index name")
}

async fn prewarm_indexes(conn: &mut PgConnection, dataset: &str, r#type: &str) {
    let prewarm_sql = format!("datasets/{dataset}/prewarm/{type}.sql");
    for statement in queries(Path::new(&prewarm_sql)) {
        sqlx::query(&statement)
            .execute(&mut *conn)
            .await
            .expect("Failed to prewarm indexes");
    }
}

/// Execute a benchmark query multiple times on a single reused connection for one query file.
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
    conn: &mut PgConnection,
    query_type: &str,
    query: &str,
    times: usize,
    fail_on_error: bool,
) -> Option<(Vec<f64>, usize)> {
    let mut results = Vec::new();
    let mut num_results = 0;

    for i in 0..times {
        let start = Instant::now();
        let result = sqlx::raw_sql(query).execute(&mut *conn).await;
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
                    return None;
                }
            }
        }
    }

    Some((results, num_results))
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
        Err("unsupported platform (cache-drop is only implemented on Linux)".to_string())
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

async fn ensure_pg_buffercache_extension(conn: &mut PgConnection) -> Result<(), String> {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_buffercache")
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            format!("failed to create pg_buffercache extension (`CREATE EXTENSION IF NOT EXISTS pg_buffercache`): {e}")
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
