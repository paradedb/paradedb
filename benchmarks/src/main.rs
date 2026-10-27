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

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use paradedb::{Window, confidence_interval_half_width, mean, percentile};
use sqlx::{AssertSqlSafe, Connection, PgConnection, Row};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

mod backrest;
mod config;
mod convert;
mod sample;
mod utils;

use config::{LoadFormat, load_dataset_config};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|err| format!("invalid positive integer: {err}"))?;
    if value == 0 {
        Err("value must be at least 1".to_string())
    } else {
        Ok(value)
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Run benchmarks against a ParadeDB instance.
    Benchmark(BenchmarkArgs),
    /// Convert parquet datasets in S3 to CSV format using DuckDB.
    Convert(convert::ConvertArgs),
    /// Sample a Parquet dataset to a target row count, preserving table relationships.
    Sample(sample::SampleArgs),
    /// Load a dataset's heap without building the index or running queries, so the resulting
    /// cluster can be captured as a snapshot (e.g. by pgBackRest).
    LoadHeap(LoadHeapArgs),
    /// Capture a loaded heap as a pgBackRest snapshot.
    SnapshotHeap(backrest::SnapshotHeapArgs),
    /// Restore a heap snapshot with pgBackRest.
    RestoreHeap(backrest::RestoreHeapArgs),
}

#[derive(Parser)]
struct BenchmarkArgs {
    /// Postgres URL.
    #[arg(long)]
    url: String,

    /// Dataset to use.
    #[arg(long, default_value = "stackoverflow")]
    dataset: String,

    /// Which index variant to build and benchmark (e.g. "bm25"). Required;
    /// resolves to `datasets/{dataset}/indexes/{index}.sql`.
    #[arg(long)]
    index: String,

    /// Dataset size label (e.g. "1m", "10m"). Used to scale size-dependent index parameters such as
    /// index options; only required for indexes that reference it.
    #[arg(long)]
    size: Option<String>,

    /// Whether to pre-warm the dataset using `pg_prewarm`.
    #[arg(long, default_value_t = true, num_args = 1)]
    prewarm: bool,

    /// Whether to run `VACUUM FULL ANALYZE`, then `VACUUM ANALYZE`, before executing queries.
    #[arg(long, default_value_t = true, num_args = 1)]
    vacuum: bool,

    /// Whether to run dirty MVCC variants for queries marked sensitive.
    #[arg(long, default_value_t = true, num_args = 1)]
    dirty: bool,

    /// Fraction of data to dirty (e.g. 0.005 for 0.5% dirty / 99.5% visible).
    #[arg(long, default_value_t = 0.005)]
    dirty_fraction: f64,

    /// Skip index creation (and the after-create-index hook). Assumes the index already exists;
    /// useful for iterating on queries against an already-indexed database.
    #[arg(long, default_value_t = false)]
    skip_index: bool,

    /// Number of runs to execute for each query.
    #[arg(long, default_value = "3", value_parser = parse_positive_usize)]
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
struct LoadHeapArgs {
    /// Postgres URL.
    #[arg(long)]
    url: String,

    /// Dataset to load.
    #[arg(long, default_value = "stackoverflow")]
    dataset: String,

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
        // The dataset's heap is assumed to already be present, restored from a snapshot before this
        // run (e.g. by the CI workflow, before Postgres started). The index is (re)built by
        // run_sql_benchmarks, gated on `--skip-index`.
        Commands::Benchmark(args) => run_sql_benchmarks(&args).await,
        Commands::Convert(args) => convert::run_convert(args),
        Commands::Sample(args) => sample::run_sample(args),
        // Load the heap without building the index or running queries, leaving a heap-only cluster
        // ready to be captured as a snapshot. The benchmark job rebuilds the index after restore.
        Commands::LoadHeap(args) => load_external_data(
            &args.url,
            &args.dataset,
            &args.size,
            args.data_source.as_deref(),
        ),
        Commands::SnapshotHeap(args) => backrest::run_snapshot_heap(args),
        Commands::RestoreHeap(args) => backrest::run_restore_heap(args),
    }
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

enum IndexCreationResult {
    Bm25 {
        duration_min_ms: f64,
        index_name: String,
        index_size: i64,
        segment_count: i64,
    },
    /// Non-bm25 access methods have no segments.
    Other {
        duration_min_ms: f64,
        index_name: String,
        index_size: i64,
    },
}

impl IndexCreationResult {
    fn index_name(&self) -> &str {
        match self {
            Self::Bm25 { index_name, .. } | Self::Other { index_name, .. } => index_name,
        }
    }

    fn duration_min_ms(&self) -> f64 {
        match self {
            Self::Bm25 {
                duration_min_ms, ..
            }
            | Self::Other {
                duration_min_ms, ..
            } => *duration_min_ms,
        }
    }

    fn index_size(&self) -> i64 {
        match self {
            Self::Bm25 { index_size, .. } | Self::Other { index_size, .. } => *index_size,
        }
    }

    /// Segment count, or `None` for access methods without segments (non-bm25).
    fn segment_count(&self) -> Option<i64> {
        match self {
            Self::Bm25 { segment_count, .. } => Some(*segment_count),
            Self::Other { .. } => None,
        }
    }
}

struct QueryResult {
    query_type: String,
    query: String,
    results: QueryRunResults,
}

#[derive(Clone)]
struct BenchmarkQuery {
    query_type: String,
    query: String,
    is_mvcc_sensitive: bool,
}

#[derive(serde::Serialize)]
struct JSONBenchmarkResult {
    name: String,
    unit: &'static str,
    value: f64,
    range: String,
    extra: String,
}

const TRACKED_PERCENTILES: [(&str, f64); 3] = [("p50", 0.50), ("p95", 0.95), ("p99", 0.99)];

impl JSONBenchmarkResult {
    fn from_query_result(res: QueryResult) -> Vec<Self> {
        let percentiles = TRACKED_PERCENTILES
            .iter()
            .map(|(label, p)| format!("{label}={:.3}", percentile(&res.results.samples, *p)))
            .collect::<Vec<_>>()
            .join("; ");
        let extra = format!(
            "cold_query_ms={:.3}; n={}; {percentiles}; query={}",
            res.results.cold,
            res.results.samples.len(),
            res.query
        );

        let mean = mean(&res.results.samples);
        let ci_half_width = confidence_interval_half_width(&res.results.samples, 0.95);
        println!(
            "Query results: {}: {mean:.3} ms ±{ci_half_width:.3} ms",
            res.query
        );
        vec![Self {
            name: res.query_type,
            unit: "mean ms",
            value: mean,
            range: format!("±{ci_half_width:.3} ms"),
            extra,
        }]
    }

    /// Build cost as tracked series, so a regression in how long an index takes to build, or how
    /// much disk it occupies, is caught the same way a query regression is.
    ///
    /// Segment count rides along in `extra` rather than becoming its own series: it is set by the
    /// index definition, so tracking it would alert on deliberate retuning rather than a regression.
    fn from_index_creation(result: &IndexCreationResult) -> [Self; 2] {
        let extra = result
            .segment_count()
            .map_or_else(String::new, |c| format!("segments={c}"));

        [
            Self {
                name: format!("{} build time", result.index_name()),
                unit: "min",
                value: result.duration_min_ms(),
                range: String::new(),
                extra: extra.clone(),
            },
            Self {
                name: format!("{} index size", result.index_name()),
                unit: "MB",
                value: result.index_size() as f64,
                range: String::new(),
                extra,
            },
        ]
    }
}

/// Nominal row count for a size label like `100k`, `1m`, `20m`.
fn dataset_rows(size: &str) -> anyhow::Result<i64> {
    let s = size.trim().to_lowercase();
    let (digits, mult) = if let Some(d) = s.strip_suffix('k') {
        (d, 1_000)
    } else if let Some(d) = s.strip_suffix('m') {
        (d, 1_000_000)
    } else {
        (s.as_str(), 1)
    };
    let n: i64 = digits
        .parse()
        .with_context(|| format!("Invalid --size label `{size}`"))?;
    Ok(n * mult)
}

/// The `{{ name }}` references in a template string.
fn template_names(s: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else { break };
        names.push(after[..close].trim().to_owned());
        rest = &after[close + 2..];
    }
    names
}

/// Replace every `{{ name }}` in `s` with `vars[name]`, erroring on an unknown name.
fn substitute_vars(s: &str, vars: &HashMap<String, String>) -> anyhow::Result<String> {
    let mut out = String::new();
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after
            .find("}}")
            .with_context(|| format!("Unterminated '{{{{' in `{s}`"))?;
        let name = after[..close].trim();
        let value = vars
            .get(name)
            .with_context(|| format!("Unknown template variable `{name}` in `{s}`"))?;
        out.push_str(value);
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Resolve the dataset's `[params]` referenced by SQL (index DDL or query files) into concrete
/// values. Each param is an expression over recognized variables (currently `dataset_size`, from
/// `--size`) and is evaluated as a SQL scalar — so the SQL stays plain (e.g. per-query probes).
async fn resolve_template_params(
    conn: &mut PgConnection,
    dataset: &str,
    size: Option<&str>,
    statements: &[String],
) -> anyhow::Result<HashMap<String, String>> {
    resolve_template_params_except(conn, dataset, size, statements, &HashSet::new()).await
}

/// As [`resolve_template_params`], but skips names supplied by the caller.
async fn resolve_template_params_except(
    conn: &mut PgConnection,
    dataset: &str,
    size: Option<&str>,
    statements: &[String],
    skip: &HashSet<String>,
) -> anyhow::Result<HashMap<String, String>> {
    let referenced: HashSet<String> = statements
        .iter()
        .flat_map(|s| template_names(s))
        .filter(|name| !skip.contains(name))
        .collect();
    if referenced.is_empty() {
        return Ok(HashMap::new());
    }

    let (config, _) = load_dataset_config(&format!("datasets/{dataset}/config.toml"))?;

    let mut vars = HashMap::new();
    if let Some(size) = size {
        vars.insert("dataset_size".to_owned(), dataset_rows(size)?.to_string());
    }

    let mut params = HashMap::new();
    for name in referenced {
        let expr = config.params.get(&name).with_context(|| {
            format!("SQL references `{{{{ {name} }}}}` but the dataset's [params] has no `{name}`")
        })?;
        let value = eval_param(conn, &name, expr, &vars).await?;
        params.insert(name, value);
    }
    Ok(params)
}

/// Substitute `vars` into a `[params]` expression and evaluate it server-side.
///
/// Evaluated through `numeric` (not `bigint`) so fractional params keep their decimals; integer
/// values still render without a trailing `.0`.
async fn eval_param(
    conn: &mut PgConnection,
    name: &str,
    expr: &str,
    vars: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let expr = substitute_vars(expr, vars).with_context(|| format!("In [params] `{name}`"))?;
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT ({expr})::numeric::text")))
        .fetch_one(&mut *conn)
        .await
        .with_context(|| format!("Failed to evaluate [params] `{name}` = `{expr}`"))
}

async fn process_index_creation(args: &BenchmarkArgs) -> anyhow::Result<Vec<IndexCreationResult>> {
    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let index_sql = format!("datasets/{}/indexes/{}.sql", args.dataset, args.index);
    let statements = queries(Path::new(&index_sql));
    let params =
        resolve_template_params(&mut conn, &args.dataset, args.size.as_deref(), &statements)
            .await?;
    let mut results = Vec::new();

    for statement in statements {
        let statement = substitute_vars(&statement, &params)?;
        println!("{statement}");

        let start = Instant::now();
        sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut conn)
            .await
            .with_context(|| "Failed to execute index creation SQL")?;
        let duration_min_ms = start.elapsed().as_secs_f64() / 60.0;

        let index_name = extract_index_name(&statement).to_owned();
        let (index_size, amname) = sqlx::query_as::<_, (i64, String)>(
            "SELECT pg_relation_size(c.oid) / (1024 * 1024), am.amname \
             FROM pg_class c JOIN pg_am am ON am.oid = c.relam WHERE c.relname = $1",
        )
        .bind(&index_name)
        .fetch_one(&mut conn)
        .await
        .with_context(|| "Failed to get index metadata")?;

        // `paradedb.index_info()` (segment count) only applies to pg_search (bm25) indexes;
        // other access methods (e.g. pgvector's hnsw/ivfflat) have no segments.
        let result = if amname == "bm25" {
            let segment_count = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT count(*) FROM paradedb.index_info('{index_name}')"
            )))
            .fetch_one(&mut conn)
            .await
            .with_context(|| "Failed to get segment count")?;
            IndexCreationResult::Bm25 {
                duration_min_ms,
                index_name,
                index_size,
                segment_count,
            }
        } else {
            IndexCreationResult::Other {
                duration_min_ms,
                index_name,
                index_size,
            }
        };
        results.push(result);
    }

    Ok(results)
}

async fn process_after_create_index_sql(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let after_create_index_sql = format!("datasets/{}/after_create_index.sql", args.dataset);
    if !Path::new(&after_create_index_sql).exists() {
        return Ok(());
    }

    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;

    // Resolve `{{ params }}` (e.g. probes, sized to the dataset) and run each statement.
    let statements = queries(Path::new(&after_create_index_sql));
    let params =
        resolve_template_params(&mut conn, &args.dataset, args.size.as_deref(), &statements)
            .await?;
    for statement in statements {
        let statement = substitute_vars(&statement, &params)?;
        sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut conn)
            .await
            .with_context(|| {
                let preview: String = statement.chars().take(60).collect();
                format!("Failed to run after_create_index statement: {preview}")
            })?;
    }
    Ok(())
}

/// Database-level GUC holding the query vector that every query file orders by.
fn format_percent(pct: f64) -> String {
    if (pct.round() - pct).abs() < 1e-6 {
        format!("{pct:.0}")
    } else {
        format!("{pct:.1}")
    }
}

async fn process_dirty_sql(
    conn: &mut PgConnection,
    args: &BenchmarkArgs,
    visible_label: &str,
) -> anyhow::Result<()> {
    let dirty_sql = format!("datasets/{}/dirty.sql", args.dataset);
    if !Path::new(&dirty_sql).exists() {
        bail!(
            "Dataset '{}' has queries marked with -- @mvcc_sensitive, but no dirtying script found at '{}'",
            args.dataset,
            dirty_sql
        );
    }

    let dirty_pct = args.dirty_fraction * 100.0;
    let dirty_pct_str = format_percent(dirty_pct);
    let modulus = (1.0 / args.dirty_fraction).round().max(1.0) as u64;

    println!(
        "Dirtying tables ({dirty_pct_str}% dirty / {visible_label}% visible, 1 tuple per {modulus} blocks)..."
    );
    let statements = queries(Path::new(&dirty_sql));

    let mut vars = HashMap::new();
    vars.insert("dirty_pct".to_string(), dirty_pct_str);
    vars.insert(
        "dirty_fraction".to_string(),
        args.dirty_fraction.to_string(),
    );
    vars.insert("dirty_modulus".to_string(), modulus.to_string());
    vars.insert("visible_pct".to_string(), visible_label.to_string());
    if let Some(size) = args.size.as_deref() {
        vars.insert("dataset_size".to_string(), dataset_rows(size)?.to_string());
    }

    let skip: HashSet<String> = vars.keys().cloned().collect();
    let dataset_params = resolve_template_params_except(
        conn,
        &args.dataset,
        args.size.as_deref(),
        &statements,
        &skip,
    )
    .await?;
    vars.extend(dataset_params);

    for statement in statements {
        let statement = substitute_vars(&statement, &vars)?;
        let start = std::time::Instant::now();
        let rows_affected = sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut *conn)
            .await
            .with_context(|| {
                let preview: String = statement.chars().take(60).collect();
                format!("Failed to run dirty statement: {preview}")
            })?
            .rows_affected();
        let preview: String = statement
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .chars()
            .take(50)
            .collect();
        println!(
            "  Executed '{preview}' in {:.2?} ({rows_affected} rows modified)",
            start.elapsed()
        );
    }

    sqlx::raw_sql("CHECKPOINT;")
        .execute(conn)
        .await
        .with_context(|| "Failed to execute checkpoint after dirtying tables.")?;

    Ok(())
}

async fn execute_benchmark_query(
    conn: &mut PgConnection,
    args: &BenchmarkArgs,
    query_type: &str,
    query: &str,
) -> anyhow::Result<Option<QueryResult>> {
    if args.clear_caches
        && let Err(err) = clear_caches(conn).await
    {
        panic!("Failed to clear caches before query: {err}");
    }

    sqlx::raw_sql("CHECKPOINT;")
        .execute(conn)
        .await
        .with_context(|| "Failed to execute checkpoint.")?;

    println!("Query Type: {query_type}\nQuery: {query}");
    let result =
        execute_query_multiple_times(&args.url, query_type, query, args.runs, args.fail_on_error)
            .await?;
    match result {
        Some(query_results) => {
            println!(
                "Results: [cold: {:?} ] {:?} | Rows Returned: {}\n",
                query_results.cold, query_results.samples, query_results.num_results
            );
            Ok(Some(QueryResult {
                query_type: query_type.to_string(),
                query: query.to_string(),
                results: query_results,
            }))
        }
        None => {
            println!("Skipped (query error)\n");
            Ok(None)
        }
    }
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
        // VACUUM FULL does not update the visibility map. Run both types so that our heap file
        // contains no wasted space and we get an updated vm
        sqlx::query("VACUUM ANALYZE")
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

    // Locate all query paths, sorted for stable output. An index may ship its own query set under
    // `queries/{index}/`; otherwise fall back to the flat `queries/` dir.
    let queries_dir = {
        let index_specific = format!("datasets/{}/queries/{}", args.dataset, args.index);
        if Path::new(&index_specific).is_dir() {
            index_specific
        } else {
            format!("datasets/{}/queries", args.dataset)
        }
    };
    // Parse each query once (reused below for execution). Resolve their `{{ param }}` references
    // (e.g. per-query probes/ef_search scaled by dataset_size) from config.toml [params], the same
    // templating used for index DDL.
    let parsed_queries = load_benchmark_queries(Path::new(&queries_dir))?;
    let query_stmts: Vec<String> = parsed_queries.iter().map(|q| q.query.clone()).collect();
    let query_params = resolve_template_params(
        &mut utility_conn,
        &args.dataset,
        args.size.as_deref(),
        &query_stmts,
    )
    .await?;

    let mut results = Vec::new();
    for query_item in &parsed_queries {
        let query = substitute_vars(&query_item.query, &query_params)?;
        if let Some(res) =
            execute_benchmark_query(&mut utility_conn, args, &query_item.query_type, &query).await?
        {
            results.push(res);
        }
    }

    let sensitive_queries: Vec<_> = parsed_queries
        .iter()
        .filter(|q| q.is_mvcc_sensitive)
        .collect();

    if args.dirty && !sensitive_queries.is_empty() && args.dirty_fraction > 0.0 {
        let visible_pct = (1.0 - args.dirty_fraction) * 100.0;
        let visible_pct_str = format_percent(visible_pct);

        process_dirty_sql(&mut utility_conn, args, &visible_pct_str).await?;

        for query_item in sensitive_queries {
            let query = substitute_vars(&query_item.query, &query_params)?;
            let dirty_query_type =
                format!("{} ({}% visible)", query_item.query_type, visible_pct_str);
            if let Some(res) =
                execute_benchmark_query(&mut utility_conn, args, &dirty_query_type, &query).await?
            {
                results.push(res);
            }
        }
    }

    results.sort_by(|a, b| a.query_type.cmp(&b.query_type));

    Ok(results)
}

async fn generate_markdown_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let output_file = "results_pg_search.md";
    let mut file = File::create(output_file).with_context(|| "Failed to create output file")?;

    write_benchmark_header(&mut file)?;
    write_test_info(&mut file, args).await?;
    write_postgres_settings(&mut file, &args.url).await?;
    if !args.skip_index {
        process_index_creation_md(&mut file, args).await?;
        process_after_create_index_sql(args).await?;
    }
    run_benchmarks_md(&mut file, args).await?;
    Ok(())
}

async fn generate_csv_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    write_test_info_csv(args).await?;
    write_postgres_settings_csv(&args.url).await?;
    if !args.skip_index {
        process_index_creation_csv(args).await?;
        process_after_create_index_sql(args).await?;
    }
    run_benchmarks_csv(args).await?;
    Ok(())
}

async fn generate_json_output(args: &BenchmarkArgs) -> anyhow::Result<()> {
    // Keep index and query metrics together, with a separate result file per configuration.
    let mut results = Vec::new();
    if !args.skip_index {
        results.extend(
            process_index_creation(args)
                .await?
                .iter()
                .flat_map(JSONBenchmarkResult::from_index_creation),
        );
        process_after_create_index_sql(args).await?;
    }
    results.extend(
        run_benchmarks(args)
            .await?
            .into_iter()
            .flat_map(JSONBenchmarkResult::from_query_result),
    );

    let size_suffix = args
        .size
        .as_deref()
        .map(|size| format!("_{size}"))
        .unwrap_or_default();
    let output_file = format!("results_{}_{}{size_suffix}.json", args.dataset, args.index);
    let mut file =
        File::create(&output_file).with_context(|| format!("Failed to create {output_file}"))?;
    let results_json =
        serde_json::to_string(&results).with_context(|| "Failed to serialize results")?;
    file.write_all(results_json.as_bytes())
        .with_context(|| "Failed to write results")?;
    Ok(())
}

/// Returns the live row count of the dataset's root table, for reporting.
async fn root_table_row_count(args: &BenchmarkArgs) -> anyhow::Result<i64> {
    let config_path = format!("datasets/{}/config.toml", args.dataset);
    let (config, _) = config::load_dataset_config(&config_path)
        .with_context(|| format!("Failed to load config '{config_path}'"))?;
    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;
    let table = &config.root_table.name;
    let row = sqlx::query(AssertSqlSafe(format!("SELECT count(*) FROM \"{table}\"")))
        .fetch_one(&mut conn)
        .await
        .with_context(|| format!("Failed to count rows in '{table}'"))?;
    Ok(row.get(0))
}

async fn write_test_info_csv(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let filename = "results_pg_search_test_info.csv";
    let mut file = File::create(filename).with_context(|| "Failed to create test info CSV")?;

    writeln!(file, "Key,Value").unwrap();
    writeln!(file, "Dataset Rows,{}", root_table_row_count(args).await?)?;
    writeln!(file, "Prewarm,{}", args.prewarm)?;
    writeln!(file, "Vacuum,{}", args.vacuum)?;
    writeln!(file, "Dirty,{}", args.dirty)?;
    writeln!(file, "Dirty Fraction,{}", args.dirty_fraction)?;

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
        let row = sqlx::query(AssertSqlSafe(format!("SHOW {setting}")))
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
        let segment_count = result
            .segment_count()
            .map_or_else(|| "-".to_string(), |c| c.to_string());
        writeln!(
            file,
            "{},{:.2},{},{}",
            result.index_name(),
            result.duration_min_ms(),
            result.index_size(),
            segment_count
        )?;
    }
    Ok(())
}

async fn run_benchmarks_csv(args: &BenchmarkArgs) -> anyhow::Result<()> {
    let filename = "results_pg_search_benchmark_results.csv";
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
    writeln!(
        file,
        "| Dataset Rows | {} |",
        root_table_row_count(args).await?
    )?;
    writeln!(file, "| Prewarm     | {} |", args.prewarm)?;
    writeln!(file, "| Vacuum      | {} |", args.vacuum)?;
    writeln!(
        file,
        "| Dirty       | {} ({}% visible) |",
        args.dirty,
        format_percent((1.0 - args.dirty_fraction) * 100.0)
    )?;

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
        let row = sqlx::query(AssertSqlSafe(format!("SHOW {setting}")))
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

    // Determine data source path.
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
        "{}/sampled/{}/{}",
        base_path.trim_end_matches('/'),
        size_label,
        config.load_format.as_str(),
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

    match config.load_format {
        LoadFormat::Csv => load_tables_csv(url, dataset, &config, &source_path)?,
        LoadFormat::Parquet => load_tables_parquet(url, &config, &source_path)?,
    }

    println!("External data loaded successfully.");
    Ok(())
}

/// Download each table's CSV files locally via DuckDB, then `psql \copy` them into Postgres.
fn load_tables_csv(
    url: &str,
    dataset: &str,
    config: &config::DatasetConfig,
    source_path: &str,
) -> anyhow::Result<()> {
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

    Ok(())
}

/// Load each table's parquet files directly into Postgres via DuckDB's postgres extension,
/// preserving native column types (e.g. embedding vectors) and float precision.
fn load_tables_parquet(
    url: &str,
    config: &config::DatasetConfig,
    source_path: &str,
) -> anyhow::Result<()> {
    let conn = utils::open_duckdb_conn().with_context(|| "Failed to open DuckDB connection")?;
    conn.execute_batch("INSTALL postgres; LOAD postgres;")
        .with_context(|| "Failed to load DuckDB postgres extension")?;
    conn.execute_batch(&format!("ATTACH '{url}' AS pg (TYPE postgres);"))
        .with_context(|| "Failed to ATTACH target Postgres from DuckDB")?;

    for table_name in config.all_table_names() {
        let glob = format!("{source_path}/{table_name}/*.parquet");
        println!("Loading '{table_name}' from {glob} into PostgreSQL...");
        conn.execute_batch(&format!(
            "INSERT INTO pg.public.\"{table_name}\" SELECT * FROM read_parquet('{glob}');"
        ))
        .with_context(|| format!("Failed to load parquet into table '{table_name}'"))?;
        println!("  Loaded '{table_name}'.");
    }

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
        let segment_count = result
            .segment_count()
            .map_or_else(|| "-".to_string(), |c| c.to_string());

        writeln!(
            file,
            "| {} | {:.2} | {} | {} |",
            result.index_name(),
            result.duration_min_ms(),
            result.index_size(),
            segment_count
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

///
/// Return a Vec of the query strings contained in the given file path.
///
/// Strip line comments and flatten a SQL statement onto one line, but keep the interior of
/// dollar-quoted ($$...$$) blocks verbatim (e.g. a TOML index `options` that needs its newlines).
/// Splitting on `$$` alternates outside/inside (even = outside, odd = inside); flatten only the outside.
fn parse_sql_statement(statement: &str) -> String {
    statement
        .split("$$")
        .enumerate()
        .map(|(i, seg)| {
            if i % 2 == 1 {
                seg.to_owned()
            } else {
                seg.split('\n')
                    .map(|line| line.split("--").next().unwrap().trim())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        })
        .collect::<Vec<_>>()
        .join("$$")
        .trim()
        .to_owned()
}

/// Strips comments and flattens each query onto a single line.
///
/// Will only split on semicolons with trailing newlines, which allows for applying GUCs to queries.
fn queries(file: &Path) -> Vec<String> {
    let content = std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("Failed to read file `{file:?}`: {e}"));

    content
        .split(";\n")
        .filter_map(|query| {
            let query = parse_sql_statement(query);
            if query.is_empty() { None } else { Some(query) }
        })
        .collect()
}

fn extract_index_name(statement: &str) -> &str {
    statement
        .split_whitespace()
        .nth(2)
        .expect("Failed to parse index name")
}

fn is_file_mvcc_sensitive(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.trim().starts_with("-- @mvcc_sensitive"))
}

/// Load all benchmark queries from `queries_dir`.
///
/// Queries can be structured in two ways:
/// 1. A nested directory `queries/{query_name}/` containing one or more `*.sql` variant files
///    (e.g. `postgres.sql`, `hash_partitioned.sql`, `range_partitioned.sql`) alongside optional
///    documentation like `README.md`. When multiple variants exist, each is labeled
///    `{query_name} - {variant_stem}`. When a single variant exists, it is labeled `{query_name}`.
/// 2. A flat `queries/{query_name}.sql` file, supported for single queries or legacy files.
///    If a directory of the same stem exists, the directory takes precedence over the flat file.
fn load_benchmark_queries(queries_dir: &Path) -> anyhow::Result<Vec<BenchmarkQuery>> {
    let mut entries: Vec<_> = std::fs::read_dir(queries_dir)
        .with_context(|| {
            format!(
                "Failed to read queries directory `{}`",
                queries_dir.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "Failed to read directory entry")?;
    entries.sort_by_key(|e| e.path());

    let mut queries = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let query_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();

            let mut sql_files: Vec<_> = std::fs::read_dir(&path)
                .with_context(|| format!("Failed to read query directory `{}`", path.display()))?
                .collect::<Result<Vec<_>, _>>()
                .with_context(|| "Failed to read directory entry")?
                .into_iter()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("sql"))
                .collect();
            sql_files.sort_unstable();

            let multi_variants = sql_files.len() > 1;
            for sql_path in sql_files {
                let stem = sql_path.file_stem().unwrap_or_default().to_string_lossy();
                let query_type = if multi_variants {
                    format!("{query_name} - {stem}")
                } else if stem == "default" || stem == "query" || stem == query_name {
                    query_name.clone()
                } else {
                    format!("{query_name} - {stem}")
                };

                let content = std::fs::read_to_string(&sql_path)
                    .unwrap_or_else(|e| panic!("Failed to read file `{sql_path:?}`: {e}"));
                let is_mvcc_sensitive = is_file_mvcc_sensitive(&content);
                let query = parse_sql_statement(&content)
                    .trim()
                    .trim_end_matches(';')
                    .trim()
                    .to_owned();
                if query.is_empty() {
                    bail!("Query file `{}` is empty", sql_path.display());
                }
                queries.push(BenchmarkQuery {
                    query_type,
                    query,
                    is_mvcc_sensitive,
                });
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            // If a directory with the same stem exists, the directory supersedes this file.
            if path.with_extension("").is_dir() {
                continue;
            }
            let query_type = path
                .file_stem()
                .unwrap_or_else(|| panic!("Failed to get file stem for `{}`", path.display()))
                .to_string_lossy()
                .into_owned();
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read file `{path:?}`: {e}"));
            let is_mvcc_sensitive = is_file_mvcc_sensitive(&content);
            let query = parse_sql_statement(&content)
                .trim()
                .trim_end_matches(';')
                .trim()
                .to_owned();
            if query.is_empty() {
                bail!("Query file `{}` is empty", path.display());
            }
            queries.push(BenchmarkQuery {
                query_type,
                query,
                is_mvcc_sensitive,
            });
        }
    }

    Ok(queries)
}

async fn prewarm_indexes(conn: &mut PgConnection, dataset: &str) -> anyhow::Result<()> {
    let prewarm_sql = format!("datasets/{dataset}/prewarm.sql");
    for statement in queries(Path::new(&prewarm_sql)) {
        sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut *conn)
            .await
            .with_context(|| "Failed to prewarm indexes")?;
    }
    Ok(())
}

async fn get_query_id(query: &str, conn: &mut PgConnection) -> anyhow::Result<i64> {
    let explain_str = format!("EXPLAIN (VERBOSE, FORMAT JSON) {query}");
    let sqlx::types::Json(res): sqlx::types::Json<serde_json::Value> =
        sqlx::query_scalar(AssertSqlSafe(explain_str.as_str()))
            .fetch_one(conn)
            .await?;

    let query_id = res[0]["Query Identifier"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("Failed to find query id"))?;

    Ok(query_id)
}

/// Run the query once and read back its server-side timing, returning `(exec + plan ms, rows)`.
async fn run_once(
    conn: &mut PgConnection,
    query: &str,
    stats_reset_query: &str,
    stats_query: &str,
) -> anyhow::Result<(f64, i64)> {
    sqlx::raw_sql(AssertSqlSafe(stats_reset_query))
        .execute(&mut *conn)
        .await
        .with_context(|| format!("Failed to execute query: {stats_reset_query}"))?;
    sqlx::raw_sql(AssertSqlSafe(query))
        .execute(&mut *conn)
        .await
        .with_context(|| format!("Failed to execute query: {query}"))?;
    let (exec_time_ms, plan_time_ms, rows): (f64, f64, i64) =
        sqlx::query_as(AssertSqlSafe(stats_query))
            .fetch_one(&mut *conn)
            .await
            .with_context(|| format!("Failed to execute query: {stats_query}"))?;
    Ok((exec_time_ms + plan_time_ms, rows))
}

/// A query that errors either aborts the run or is skipped, depending on `--fail-on-error`.
fn on_query_error(
    query_type: &str,
    err: anyhow::Error,
    fail_on_error: bool,
) -> anyhow::Result<Option<QueryRunResults>> {
    if fail_on_error {
        panic!("Failed to execute benchmark query `{query_type}`:  {err}");
    }
    eprintln!("WARNING: Skipping query `{query_type}` due to error: {err}");
    Ok(None)
}

/// Execute a benchmark query, taking sample_count warmed samples on a single reused connection.
///
/// This creates a fresh connection for each benchmark query and then reuses it across repeated
/// runs of that query.
///
/// The query runs repeatedly, warming it, until a 3-run window shows a sub-0.1% ratio of
/// variance over mean, or it has run 10 times. At that point, sample_count samples will
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

    let measured_query = query
        .split(';')
        .map(str::trim)
        .rfind(|s| !s.is_empty())
        .unwrap();
    // SELECT the times for the last query run, making sure we don't accidentally get the 'reset'
    // query
    let stats_reset_query = "SELECT pg_stat_statements_reset();";

    // Apply the query's `SET` preamble to the session first, so operating-point GUCs are in effect for
    // get_query_id's EXPLAIN of the bare measured query below (some access methods error at plan time
    // when a required GUC is unset). Idempotent -- the measured runs re-apply them via the full query.
    for stmt in query.split(';') {
        let stmt = stmt.trim();
        if stmt.len() >= 4 && stmt[..4].eq_ignore_ascii_case("set ") {
            sqlx::raw_sql(AssertSqlSafe(format!("{stmt};")))
                .execute(&mut conn)
                .await
                .with_context(|| format!("Failed to apply query setting: {stmt}"))?;
        }
    }

    let query_id = get_query_id(measured_query, &mut conn).await?;
    let stats_query = format!(
        "SELECT max_exec_time, max_plan_time, rows FROM pg_stat_statements WHERE queryid = {query_id};"
    );

    // Log the plan the timings will measure, so a benchmark run's logs show which execution
    // path (serial, PG-parallel Gather, MPP DistributedExec) each query took. ANALYZE makes
    // the render reflect the EXECUTED plan: launch-time fallbacks (the MPP size gate, a short
    // worker launch) replan after the plain-EXPLAIN render would have shown a distributed
    // shape. The compound statement runs once first so the plan reflects the query's own GUC
    // prefix.
    {
        use sqlx::Row;
        sqlx::raw_sql(AssertSqlSafe(query))
            .execute(&mut conn)
            .await
            .ok();
        // VERBOSE because the JoinScan renderer only includes per-node `elapsed_compute`
        // timing for verbose EXPLAINs.
        let explain = format!(
            "EXPLAIN (ANALYZE, VERBOSE, COSTS ON, TIMING OFF, SUMMARY OFF) {measured_query}"
        );
        match sqlx::raw_sql(AssertSqlSafe(explain.as_str()))
            .fetch_all(&mut conn)
            .await
        {
            Ok(rows) => {
                println!("plan for `{query_type}`:");
                for row in rows {
                    match row.try_get::<String, _>(0) {
                        Ok(line) => println!("    {line}"),
                        Err(e) => println!("    <row decode failed: {e}>"),
                    }
                }
            }
            Err(e) => println!("plan for `{query_type}`: EXPLAIN failed: {e}"),
        }
    }

    // run until run-to-run variance is sub-0.1% (query is warmed) or
    // until 10 runs have passed, then take the next sample_count results
    let mut runs_completed = 0;
    let mut samples_taken = 0;
    while samples_taken < sample_count {
        let result = run_once(&mut conn, query, stats_reset_query, &stats_query).await;

        match result {
            Ok((time, rows)) => {
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
            Err(err) => return on_query_error(query_type, err, fail_on_error),
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
        .with_context(|| format!("Failed to evict PostgreSQL buffer cache: {evict_query}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Datasets that repeat one query have gh-pages history under the bare query name, and their
    /// samples are not a latency distribution. Renaming or re-valuing them would orphan that.
    #[test]
    fn repeated_query_results_still_publish_a_single_mean_series() {
        let out = JSONBenchmarkResult::from_query_result(QueryResult {
            query_type: "count".to_owned(),
            query: "SELECT count(*) FROM t".to_owned(),
            results: QueryRunResults {
                cold: 11.108,
                samples: vec![10.0, 12.0, 14.0],
                num_results: 1,
            },
        });

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "count");
        assert_eq!(out[0].unit, "mean ms");
        assert_eq!(out[0].value, 12.0);
        assert!(out[0].extra.contains("p50=12.000"), "{}", out[0].extra);
    }

    #[test]
    fn bm25_index_publishes_build_time_and_size_with_segments_in_extra() {
        let series = JSONBenchmarkResult::from_index_creation(&IndexCreationResult::Bm25 {
            duration_min_ms: 4.25,
            index_name: "search_idx".to_owned(),
            index_size: 1234,
            segment_count: 8,
        });

        assert_eq!(series[0].name, "search_idx build time");
        assert_eq!(series[0].unit, "min");
        assert_eq!(series[0].value, 4.25);
        assert_eq!(series[1].name, "search_idx index size");
        assert_eq!(series[1].unit, "MB");
        assert_eq!(series[1].value, 1234.0);
        assert!(series.iter().all(|s| s.extra == "segments=8"));
    }

    /// Other access methods have no segments to report.
    #[test]
    fn non_bm25_index_publishes_the_same_series_without_segments() {
        let series = JSONBenchmarkResult::from_index_creation(&IndexCreationResult::Other {
            duration_min_ms: 12.5,
            index_name: "btree_idx".to_owned(),
            index_size: 40960,
        });

        assert_eq!(
            series.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["btree_idx build time", "btree_idx index size"]
        );
        assert!(series.iter().all(|s| s.extra.is_empty()));
    }

    /// github-action-benchmark's customSmallerIsBetter parser keys off these exact field names; a
    /// rename would publish entries it silently drops.
    #[test]
    fn results_json_uses_the_field_names_the_publish_action_expects() {
        let json = serde_json::to_string(
            &JSONBenchmarkResult::from_index_creation(&IndexCreationResult::Other {
                duration_min_ms: 1.0,
                index_name: "i".to_owned(),
                index_size: 2,
            })[0],
        )
        .unwrap();

        assert_eq!(
            json,
            r#"{"name":"i build time","unit":"min","value":1.0,"range":"","extra":""}"#
        );
    }

    #[test]
    fn rejects_zero_benchmark_samples() {
        assert!(
            Cli::try_parse_from([
                "benchmarks",
                "benchmark",
                "--url",
                "postgresql://localhost/postgres",
                "--index",
                "bm25",
                "--runs",
                "0",
            ])
            .is_err()
        );
    }

    #[test]
    fn parse_sql_statement_strips_comments_and_preserves_dollar_quotes() {
        let sql = r#"
-- Line comment 1
SET work_mem TO '4GB'; -- Inline comment
SET paradedb.options = $$
multiline
options
$$;
SELECT * FROM table;
"#;
        assert_eq!(
            parse_sql_statement(sql),
            "SET work_mem TO '4GB'; SET paradedb.options =$$\nmultiline\noptions\n$$; SELECT * FROM table;"
        );
    }

    #[test]
    fn detects_mvcc_sensitive_annotation() {
        assert!(is_file_mvcc_sensitive("-- @mvcc_sensitive\nSELECT 1;"));
        assert!(is_file_mvcc_sensitive(
            "-- Some comment\n  -- @mvcc_sensitive\nSELECT 1;"
        ));
        assert!(!is_file_mvcc_sensitive("-- Just a normal query\nSELECT 1;"));
    }

    #[test]
    fn parses_dirty_cli_args() {
        let cli = Cli::try_parse_from([
            "benchmarks",
            "benchmark",
            "--url",
            "postgresql://localhost/postgres",
            "--index",
            "bm25",
        ])
        .unwrap();

        match cli.command {
            Commands::Benchmark(args) => {
                assert!(args.dirty);
                assert_eq!(args.dirty_fraction, 0.005);
            }
            _ => panic!("Expected Commands::Benchmark"),
        }

        let cli = Cli::try_parse_from([
            "benchmarks",
            "benchmark",
            "--url",
            "postgresql://localhost/postgres",
            "--index",
            "bm25",
            "--dirty",
            "false",
            "--dirty-fraction",
            "0.10",
        ])
        .unwrap();

        match cli.command {
            Commands::Benchmark(args) => {
                assert!(!args.dirty);
                assert_eq!(args.dirty_fraction, 0.10);
            }
            _ => panic!("Expected Commands::Benchmark"),
        }
    }

    #[test]
    fn formats_percent_cleanly() {
        assert_eq!(format_percent(99.5), "99.5");
        assert_eq!(format_percent(95.0), "95");
        assert_eq!(format_percent(99.0), "99");
        assert_eq!(format_percent(0.5), "0.5");
        assert_eq!(format_percent(0.0), "0");
        assert_eq!(format_percent(100.0), "100");
    }
}
