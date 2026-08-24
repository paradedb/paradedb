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
use paradedb::{
    Window, confidence_interval_half_width, fnv1a, mean, percentile, percentile_confidence_interval,
};
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

use config::{DatasetConfig, LoadFormat, SweepConfig, load_dataset_config};

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
    /// Measure recall@k of a built vector index against a held-out query set (cohere).
    Recall(RecallArgs),
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

    /// Which index variant to build and benchmark (e.g. "bm25", "hnsw", "ivfflat"). Required;
    /// resolves to `datasets/{dataset}/indexes/{index}.sql`.
    #[arg(long)]
    index: String,

    /// Dataset size label (e.g. "1m", "10m"). Used to scale size-dependent index parameters such as
    /// ivfflat's `lists`; only required for indexes that reference it.
    #[arg(long)]
    size: Option<String>,

    /// Whether to pre-warm the dataset using `pg_prewarm`.
    #[arg(long, default_value_t = true, num_args = 1)]
    prewarm: bool,

    /// Whether to run `VACUUM FULL ANALYZE`, then `VACUUM ANALYZE`, before executing queries.
    #[arg(long, default_value_t = true, num_args = 1)]
    vacuum: bool,

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

#[derive(Parser)]
struct RecallArgs {
    /// Postgres URL. The corpus and its vector index are assumed to already exist (built by a prior
    /// `benchmark` run), so recall measures that exact index.
    #[arg(long)]
    url: String,

    /// Dataset to measure recall for.
    #[arg(long, default_value = "cohere")]
    dataset: String,

    /// Dataset size label (e.g. "1m", "10m"). Selects the precomputed ground-truth parquet
    /// (`{data_source}/queries/ground_truth_{query}_{size}.parquet`), which is query- and
    /// corpus-size-specific.
    #[arg(long)]
    size: String,

    /// Base path to the held-out query + ground-truth parquets (S3 or local). Overrides s3_base_path
    /// in config.toml; files load from `{data_source}/queries/`.
    #[arg(long)]
    data_source: Option<String>,

    /// Query file (stem of `queries/{query}.sql`) to measure recall for, run for each held-out
    /// vector. May include an index subdirectory (e.g. `foo/knn_top10_1pct`).
    #[arg(long, default_value = "knn_top10_unfiltered")]
    query: String,

    /// Ground-truth stem, selecting `ground_truth_{ground_truth}_{size}.parquet`. The exact top-10
    /// depends only on filter selectivity, so query files with the same filter share one ground
    /// truth. Defaults to `--query` with any index subdirectory stripped.
    #[arg(long)]
    ground_truth: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        // The dataset's heap is assumed to already be present, restored from a snapshot before this
        // run (e.g. by the CI workflow, before Postgres started). The index is (re)built by
        // run_sql_benchmarks, gated on `--skip-index`.
        Commands::Benchmark(args) => run_sql_benchmarks(&args).await,
        Commands::Recall(args) => run_recall(&args).await,
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
    /// True when each sample is a distinct query vector rather than a repeat of one. Only then is
    /// the sample set a latency distribution, so only then are percentiles meaningful.
    pub per_vector: bool,
    /// Hash of the held-out query vectors, in sample order. Published alongside the samples so a
    /// later run can pair its samples with a baseline's index-by-index, and detect when the vector
    /// set changed and pairing would be meaningless.
    pub vector_hash: Option<u64>,
}

enum IndexCreationResult {
    Bm25 {
        duration_min_ms: f64,
        index_name: String,
        index_size: i64,
        segment_count: i64,
    },
    /// Non-bm25 access methods (e.g. pgvector hnsw/ivfflat) have no segments.
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

#[derive(serde::Serialize)]
struct JSONBenchmarkResult {
    name: String,
    unit: &'static str,
    value: f64,
    range: String,
    extra: String,
}

/// Percentiles published as their own tracked series. Each becomes a separate gh-pages entry, so
/// a regression alert fires on the percentile rather than on a mean that hides the tail.
const TRACKED_PERCENTILES: [(&str, f64); 3] = [("p50", 0.50), ("p95", 0.95), ("p99", 0.99)];

impl JSONBenchmarkResult {
    /// One entry per tracked percentile when the samples are a distribution over query vectors;
    /// otherwise a single mean entry, which is what datasets that repeat one query have always
    /// reported.
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

        if !res.results.per_vector {
            let mean = mean(&res.results.samples);
            let ci_half_width = confidence_interval_half_width(&res.results.samples, 0.95);
            println!(
                r"Query results: |
            query: {},
            mean: {mean:.3} ms,
            confidence interval: ±{ci_half_width:.3} ms",
                res.query
            );
            return vec![Self {
                name: res.query_type,
                unit: "mean ms",
                value: mean,
                range: format!("±{ci_half_width:.3} ms"),
                extra,
            }];
        }

        println!(
            r"Query results: |
            query: {},
            {percentiles} (ms, over {} query vectors)",
            res.query,
            res.results.samples.len()
        );

        // The raw per-vector samples ride along on the p50 entry (once per operating point, not
        // per percentile), so a later run can pair its samples with this one's by index and test
        // per-query latency ratios instead of comparing independent percentile estimates.
        let samples_prefix = res.results.vector_hash.map(|hash| {
            let samples = res
                .results
                .samples
                .iter()
                .map(|v| format!("{v:.3}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("samples_fnv={hash:016x}; samples=[{samples}]; ")
        });

        TRACKED_PERCENTILES
            .iter()
            .map(|(label, p)| {
                let (lo, hi) = percentile_confidence_interval(&res.results.samples, *p, 0.95);
                let extra = match (*label, &samples_prefix) {
                    ("p50", Some(prefix)) => format!("{prefix}{extra}"),
                    _ => extra.clone(),
                };
                Self {
                    // " - " is the dashboard's chart-grouping separator, so an operating point's
                    // three percentiles share one chart instead of getting one each.
                    name: format!("{} - {label}", res.query_type),
                    unit: "ms",
                    value: percentile(&res.results.samples, *p),
                    range: format!("95% CI [{lo:.3}, {hi:.3}]"),
                    extra,
                }
            })
            .collect()
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

/// As [`resolve_template_params`], but skips names the caller supplies itself -- a swept param is
/// provided per point by the sweep, so it needs no `[params]` entry.
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

/// Split a query file's variant into its operating-point `SET`s and the query itself. Handles the
/// inline `SET ...; SET ...; SELECT ...` compound the harness uses.
fn split_operating_point(variant: &str) -> anyhow::Result<(Vec<String>, String)> {
    let mut sets = Vec::new();
    let mut query = None;
    for part in variant.split(';') {
        let part = part.trim().to_string();
        if part.is_empty() {
            continue;
        }
        if part.to_uppercase().starts_with("SET ") {
            sets.push(part);
        } else {
            query = Some(part);
        }
    }
    let query = query.with_context(|| "query variant has no statement to run")?;
    Ok((sets, query))
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
const QVEC_GUC: &str = "cohere.qvec";
/// Number of neighbors per query the ground truth stores (recall@k).
const RECALL_K: usize = 10;

/// Measure recall@k of an already-built vector index for one query file. Recall runs the *actual*
/// latency query (`queries/{query}.sql`) verbatim -- including its `current_setting('cohere.qvec')`
/// operand and any `SET` lines -- once per held-out vector, setting `cohere.qvec` to that vector
/// each time. Running the query with the vector as a per-call *constant* (not a join parameter) is
/// what keeps recall's query plan identical to the benchmark's: a lateral parameter can tip the
/// planner to a different plan (e.g. a btree/GIN pre-filter + exact sort instead of the ANN index),
/// which would make recall measure something the benchmark never runs. The returned top-k is
/// intersected with the precomputed exact top-k in `ground_truth_{query}_{size}.parquet`. Assumes
/// the corpus and its index already exist (from a prior `benchmark` run).
async fn run_recall(args: &RecallArgs) -> anyhow::Result<()> {
    let recall_sql = format!("datasets/{}/recall.sql", args.dataset);
    if !Path::new(&recall_sql).exists() {
        bail!("Dataset '{}' has no recall.sql", args.dataset);
    }
    let query_file = format!("datasets/{}/queries/{}.sql", args.dataset, args.query);
    if !Path::new(&query_file).exists() {
        bail!("No query file at {query_file}; --query must name a queries/<query>.sql file");
    }
    let config_path = format!("datasets/{}/config.toml", args.dataset);
    let (config, _) = config::load_dataset_config(&config_path)
        .with_context(|| format!("Failed to load config '{config_path}'"))?;

    let base = args
        .data_source
        .as_deref()
        .or(config.s3_base_path.as_deref())
        .with_context(|| {
            format!(
                "Dataset '{}' has no S3 base path. Provide --data-source or set s3_base_path in \
                 datasets/{}/config.toml",
                args.dataset, args.dataset
            )
        })?;
    let base = base.trim_end_matches('/');

    let mut conn = PgConnection::connect(&args.url)
        .await
        .with_context(|| "Failed to connect to database")?;

    // Parse the query file: resolve its `{{ param }}` references (per-query probes/ef_search scaled
    // by dataset_size), then split into the `SET` statements (operating point, applied to the
    // session) and the single kNN query. A file may hold multiple variants (the benchmark runs all
    // of them -- see benchmark_queries); recall measures only the FIRST variant (the one the
    // benchmark labels with the bare file stem), so later variants' queries and SETs don't leak in.
    // Splitting on `;` handles the inline `SET ...; SELECT ...` compound the harness uses. The query
    // is run verbatim per held-out vector, so it must order by current_setting('cohere.qvec') --
    // that lets recall vary the vector without changing the query (and thus its plan).
    let raw_statements = queries(Path::new(&query_file));
    let params =
        resolve_template_params(&mut conn, &args.dataset, Some(&args.size), &raw_statements)
            .await?;
    let first_variant = raw_statements
        .first()
        .with_context(|| format!("Query file {query_file} is empty"))?;
    let (set_statements, knn_query) =
        split_operating_point(&substitute_vars(first_variant, &params)?)
            .with_context(|| format!("Query file {query_file} has no query to score"))?;
    if !knn_query.contains(&format!("current_setting('{QVEC_GUC}')")) {
        bail!(
            "Query file {query_file} does not order by current_setting('{QVEC_GUC}'); recall cannot \
             vary the query vector for it"
        );
    }

    // Ground-truth stem: explicit --ground-truth, else --query with any index subdirectory stripped.
    let gt_stem = args
        .ground_truth
        .clone()
        .unwrap_or_else(|| args.query.rsplit('/').next().unwrap().to_string());

    let fixtures = load_recall_fixtures(
        &mut conn,
        &args.url,
        &args.dataset,
        &args.size,
        &gt_stem,
        base,
    )
    .await?;

    // Apply the query file's SET statements so recall runs at the latency query's operating point.
    for stmt in &set_statements {
        sqlx::query(AssertSqlSafe(stmt.as_str()))
            .execute(&mut conn)
            .await
            .with_context(|| format!("Failed to apply query setting: {stmt}"))?;
    }

    let recall = score_recall(&mut conn, &knn_query, &fixtures).await?;
    println!("recall = {recall:.4}");
    Ok(())
}

/// Held-out query vectors (as pgvector text, ready to assign to `cohere.qvec`) and the exact
/// ground truth, keyed by query id. Loaded once per (query, size) and reused across every recall
/// measurement at that point, so a sweep re-scores the same fixtures at each operating point
/// instead of re-reading them from S3.
struct RecallFixtures {
    vectors: Vec<(i32, String)>,
    ground_truth: HashMap<i32, HashSet<String>>,
}

/// Create `recall.sql`'s fixture tables, load each from its exact parquet key, and read them back.
async fn load_recall_fixtures(
    conn: &mut PgConnection,
    url: &str,
    dataset: &str,
    size: &str,
    gt_stem: &str,
    base: &str,
) -> anyhow::Result<RecallFixtures> {
    let recall_sql = format!("datasets/{dataset}/recall.sql");

    // Tables recall.sql creates, each loaded from an exact parquet key (not a glob, so a
    // public-GetObject bucket can be read cross-account without ListBucket) once it appears.
    let mut fixtures = vec![
        (
            "cohere_queries",
            format!("{base}/queries/cohere_queries.parquet"),
        ),
        (
            "recall_gt",
            format!("{base}/queries/ground_truth_{gt_stem}_{size}.parquet"),
        ),
    ];

    // Create the fixture tables (recall.sql) and load each from parquet right after its CREATE.
    // (Keying on the CREATE statement, not table existence, avoids loading a leftover table from a
    // prior run before recall.sql drops/recreates it.)
    for statement in queries(Path::new(&recall_sql)) {
        sqlx::query(AssertSqlSafe(statement.as_str()))
            .execute(&mut *conn)
            .await
            .with_context(|| format!("Failed to run recall setup statement: {statement}"))?;
        let is_create = statement.to_lowercase().contains("create table");
        let mut pending = Vec::new();
        for (table, source) in fixtures {
            if is_create && statement.contains(table) {
                println!("Loading {table} from {source}...");
                load_parquet_into(url, table, &source)?;
            } else {
                pending.push((table, source));
            }
        }
        fixtures = pending;
    }
    if !fixtures.is_empty() {
        let missing: Vec<_> = fixtures.iter().map(|(t, _)| *t).collect();
        bail!(
            "recall.sql did not create expected table(s): {}",
            missing.join(", ")
        );
    }

    let vectors: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, emb::text FROM cohere_queries ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .with_context(|| "Failed to read cohere_queries")?;
    let ground_truth: HashMap<i32, HashSet<String>> =
        sqlx::query_as::<_, (i32, Vec<String>)>("SELECT query_id, gt_ids FROM recall_gt")
            .fetch_all(&mut *conn)
            .await
            .with_context(|| "Failed to read recall_gt")?
            .into_iter()
            .map(|(id, ids)| (id, ids.into_iter().collect()))
            .collect();

    Ok(RecallFixtures {
        vectors,
        ground_truth,
    })
}

/// Score recall@k of `knn_query` against `fixtures`. For each held-out vector, set `cohere.qvec`
/// then run the latency query verbatim via the simple protocol (matching the benchmark, so the
/// planner picks the same plan), and intersect its top-k with the exact ground truth. The
/// session's operating-point `SET`s must already be applied.
async fn score_recall(
    conn: &mut PgConnection,
    knn_query: &str,
    fixtures: &RecallFixtures,
) -> anyhow::Result<f64> {
    let mut total_hits = 0usize;
    for (id, emb_text) in &fixtures.vectors {
        sqlx::raw_sql(AssertSqlSafe(format!("SET {QVEC_GUC} = '{emb_text}';")))
            .execute(&mut *conn)
            .await
            .with_context(|| format!("Failed to set {QVEC_GUC} for query {id}"))?;
        let rows = sqlx::raw_sql(AssertSqlSafe(knn_query))
            .fetch_all(&mut *conn)
            .await
            .with_context(|| format!("Failed to run recall query for query {id}"))?;
        let gt = fixtures
            .ground_truth
            .get(id)
            .with_context(|| format!("No ground truth for query id {id}"))?;
        for row in &rows {
            let neighbor: String = row.try_get("_id").with_context(|| {
                format!("recall query for {id} returned a row with no text `_id` column")
            })?;
            if gt.contains(&neighbor) {
                total_hits += 1;
            }
        }
    }
    Ok(total_hits as f64 / (fixtures.vectors.len() * RECALL_K) as f64)
}

/// Recall targets a sweep reports, lowest first.
const RECALL_TARGETS: [(&str, f64); 3] = [("r90", 0.90), ("r95", 0.95), ("r99", 0.99)];

/// One measured point of a sweep.
struct SweepPoint {
    /// The swept value, as written in `[sweeps].values`.
    value: String,
    recall: f64,
    /// The variant (SETs + query) with `value` substituted, ready to benchmark.
    query: String,
}

/// The point chosen for one recall target.
struct SelectedPoint {
    label: &'static str,
    target: f64,
    value: String,
    recall: f64,
    query: String,
    /// False when no swept value reached the target. The closest (highest-recall) point is
    /// reported instead, so too narrow a sweep range is visible rather than passing as a hit.
    reached: bool,
}

/// For each target, the cheapest point that reaches it.
///
/// Recall rises monotonically with these knobs, so the cheapest qualifying point is the
/// lowest-recall one at or above the target -- which avoids having to order the swept values
/// themselves, since they are opaque SQL expressions.
fn select_points(points: &[SweepPoint]) -> Vec<SelectedPoint> {
    let mut by_recall: Vec<&SweepPoint> = points.iter().collect();
    by_recall.sort_by(|a, b| a.recall.total_cmp(&b.recall));

    RECALL_TARGETS
        .iter()
        .filter_map(|(label, target)| {
            let (point, reached) = match by_recall.iter().find(|p| p.recall >= *target) {
                Some(point) => (*point, true),
                None => (*by_recall.last()?, false),
            };
            Some(SelectedPoint {
                label,
                target: *target,
                value: point.value.clone(),
                recall: point.recall,
                query: point.query.clone(),
                reached,
            })
        })
        .collect()
}

/// Measure recall at each value of `sweep` for one query, reusing a single set of fixtures.
///
/// Values are ordered cheapest first, so the sweep stops after the first value that reaches the
/// highest recall target: every target already has its cheapest qualifying point, and larger
/// values only cost more.
async fn sweep_query(
    conn: &mut PgConnection,
    args: &BenchmarkArgs,
    config: &DatasetConfig,
    size: &str,
    query_stem: &str,
    raw_variant: &str,
    sweep: &SweepConfig,
) -> anyhow::Result<Vec<SweepPoint>> {
    // The exact top-10 depends only on filter selectivity, so variants of the same filter share
    // one ground truth (vchord ships prefilter on/off variants of each).
    let gt_stem = query_stem
        .strip_suffix("_prefilter_on")
        .or_else(|| query_stem.strip_suffix("_prefilter_off"))
        .unwrap_or(query_stem);
    let base = config.s3_base_path.as_deref().with_context(|| {
        format!(
            "Sweeping `{query_stem}` needs ground truth, but dataset '{}' has no s3_base_path",
            args.dataset
        )
    })?;
    let fixtures = load_recall_fixtures(
        conn,
        &args.url,
        &args.dataset,
        size,
        gt_stem,
        base.trim_end_matches('/'),
    )
    .await?;

    if !template_names(raw_variant).contains(&sweep.param) {
        bail!(
            "Sweep for `{query_stem}` varies `{0}`, but that query does not reference `{{{{ {0} }}}}`",
            sweep.param
        );
    }
    // Resolve the variant's *other* params once; the swept one comes from the sweep per point, so
    // it needs no `[params]` entry of its own.
    let swept = HashSet::from([sweep.param.clone()]);
    let base_params = resolve_template_params_except(
        conn,
        &args.dataset,
        Some(size),
        &[raw_variant.to_owned()],
        &swept,
    )
    .await?;
    let vars = HashMap::from([("dataset_size".to_owned(), dataset_rows(size)?.to_string())]);
    let (_, top_target) = RECALL_TARGETS[RECALL_TARGETS.len() - 1];

    let mut points = Vec::new();
    for value in &sweep.values {
        let resolved = eval_param(conn, &sweep.param, value, &vars).await?;
        let mut params = base_params.clone();
        params.insert(sweep.param.clone(), resolved);

        let query = substitute_vars(raw_variant, &params)?;
        let (sets, knn_query) = split_operating_point(&query)?;
        for stmt in &sets {
            sqlx::query(AssertSqlSafe(stmt.as_str()))
                .execute(&mut *conn)
                .await
                .with_context(|| format!("Failed to apply sweep setting: {stmt}"))?;
        }
        let recall = score_recall(conn, &knn_query, &fixtures).await?;
        println!("  {} = {value} -> recall {recall:.4}", sweep.param);
        points.push(SweepPoint {
            value: value.clone(),
            recall,
            query,
        });
        if recall >= top_target {
            println!("  reached {top_target:.2} recall; skipping larger values");
            break;
        }
    }
    Ok(points)
}

/// One selected operating point, held until latency has been measured for it so the run's summary
/// can report the recall and the latency of the same point together.
struct SweepSummaryRow {
    query: String,
    label: &'static str,
    param: String,
    value: String,
    recall: f64,
    reached: bool,
}

/// Replace each query that declares a sweep with one entry per recall target, so only the selected
/// operating points are benchmarked for latency. Queries without a sweep pass through untouched.
///
/// Returns the rows for the run's sweep summary; latency is joined on afterwards by
/// [`write_sweep_summary`].
async fn expand_sweeps(
    conn: &mut PgConnection,
    args: &BenchmarkArgs,
    parsed: Vec<(String, String)>,
) -> anyhow::Result<(Vec<(String, String)>, Vec<SweepSummaryRow>)> {
    let (config, _) = load_dataset_config(&format!("datasets/{}/config.toml", args.dataset))?;
    if config.sweeps.is_empty() {
        return Ok((parsed, Vec::new()));
    }
    let size = args
        .size
        .as_deref()
        .with_context(|| "Sweeping needs --size to select the ground truth")?;

    let mut expanded = Vec::new();
    let mut summary = Vec::new();
    for (query_type, query) in parsed {
        // Only a file's first variant is labeled with the bare stem, so later variants (which
        // recall does not score) never match a sweep.
        let Some(sweep) = config.sweep_for(&args.index, &query_type) else {
            expanded.push((query_type, query));
            continue;
        };

        println!(
            "Sweeping {query_type} over {} ({} values)",
            sweep.param,
            sweep.values.len()
        );
        let points = sweep_query(conn, args, &config, size, &query_type, &query, sweep).await?;
        for selected in select_points(&points) {
            if !selected.reached {
                println!(
                    "  WARNING: no value reached {:.0}% recall for {query_type}; \
                     reporting the best point ({:.4}). Widen the sweep range.",
                    selected.target * 100.0,
                    selected.recall
                );
            }
            summary.push(SweepSummaryRow {
                query: query_type.clone(),
                label: selected.label,
                param: sweep.param.clone(),
                value: selected.value,
                recall: selected.recall,
                reached: selected.reached,
            });
            expanded.push((format!("{query_type}@{}", selected.label), selected.query));
        }
    }
    Ok((expanded, summary))
}

/// Write `sweep_summary_{size}.tsv` for the workflow to render: each selected point with the recall
/// it achieved and the latency measured at it.
fn write_sweep_summary(
    size: &str,
    rows: &[SweepSummaryRow],
    results: &[QueryResult],
) -> anyhow::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut out = Vec::new();
    for r in rows {
        // Latency was measured under the expanded label the sweep emitted.
        let benchmarked = format!("{}@{}", r.query, r.label);
        let latencies = results
            .iter()
            .find(|q| q.query_type == benchmarked)
            .map(|q| {
                TRACKED_PERCENTILES
                    .iter()
                    .map(|(_, p)| format!("{:.2}", percentile(&q.results.samples, *p)))
                    .collect::<Vec<_>>()
            })
            // A query that errored is skipped by the benchmark loop, so it has no latency.
            .unwrap_or_else(|| vec!["n/a".to_owned(); TRACKED_PERCENTILES.len()]);
        out.push(format!(
            "{}\t{}\t{}\t{}\t{:.4}\t{}\t{}",
            r.query,
            r.label,
            r.param,
            r.value,
            r.recall,
            r.reached,
            latencies.join("\t")
        ));
    }
    // Per size, so a later size's run does not clobber an earlier one's summary.
    let path = format!("sweep_summary_{size}.tsv");
    std::fs::write(&path, out.join("\n") + "\n").with_context(|| format!("Failed to write {path}"))
}

/// Load `source` (a parquet path/URL) into the already-created Postgres `table` via DuckDB's
/// postgres extension, preserving native column types (embedding vectors, text arrays).
fn load_parquet_into(url: &str, table: &str, source: &str) -> anyhow::Result<()> {
    let conn = utils::open_duckdb_conn().with_context(|| "Failed to open DuckDB connection")?;
    conn.execute_batch("INSTALL postgres; LOAD postgres;")
        .with_context(|| "Failed to load DuckDB postgres extension")?;
    conn.execute_batch(&format!("ATTACH '{url}' AS pg (TYPE postgres);"))
        .with_context(|| "Failed to ATTACH target Postgres from DuckDB")?;
    conn.execute_batch(&format!(
        "INSERT INTO pg.public.\"{table}\" SELECT * FROM read_parquet('{source}');"
    ))
    .with_context(|| format!("Failed to load '{table}' from '{source}'"))?;
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

    // Parse each query file once (reused below for execution). Resolve their `{{ param }}` references
    // (e.g. per-query probes/ef_search scaled by dataset_size) from config.toml [params], the same
    // templating used for index DDL.
    let parsed_queries: Vec<(String, String)> = query_paths
        .iter()
        .flat_map(|p| benchmark_queries(p))
        .collect();
    // Expand any query declaring a sweep into one entry per recall target, measured against the
    // index just built. Swept entries come back fully substituted; the rest still hold `{{ }}`
    // references and are resolved below.
    let (parsed_queries, sweep_summary) =
        expand_sweeps(&mut utility_conn, args, parsed_queries).await?;
    // Measure latency across every held-out query vector when this dataset has them.
    let query_vectors = load_query_vectors(&mut utility_conn).await;
    if !query_vectors.is_empty() {
        println!(
            "Measuring latency over {} held-out query vectors",
            query_vectors.len()
        );
    }
    let query_stmts: Vec<String> = parsed_queries.iter().map(|(_, q)| q.clone()).collect();
    let query_params = resolve_template_params(
        &mut utility_conn,
        &args.dataset,
        args.size.as_deref(),
        &query_stmts,
    )
    .await?;

    let mut results = Vec::new();
    for (query_type, query) in parsed_queries {
        let query = substitute_vars(&query, &query_params)?;
        if args.clear_caches
            && let Err(err) = clear_caches(&mut utility_conn).await
        {
            panic!("Failed to clear caches before query: {err}");
        }

        sqlx::raw_sql("CHECKPOINT;")
            .execute(&mut utility_conn)
            .await
            .with_context(|| "Failed to execute checkpoint.")?;

        println!("Query Type: {query_type}\nQuery: {query}");
        // Alternatives are fixed reference plans -- the exact pre-filter, say -- never a swept
        // operating point, so nothing builds a percentile series from them. Sampling one once per
        // held-out vector costs hours at 10m rows to produce a distribution no series reads. They
        // still read the vector GUC, so they get the vectors and just do not iterate them.
        let result = execute_query_multiple_times(
            &args.url,
            &query_type,
            &query,
            args.runs,
            args.fail_on_error,
            &query_vectors,
            !is_alternative(&query_type),
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

    if let Some(size) = args.size.as_deref() {
        write_sweep_summary(size, &sweep_summary, &results)?;
    }

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
    // Index and query metrics share one results.json, so the publish step reports them as one set.
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

    let mut file = File::create("results.json").with_context(|| "Failed to create output file")?;
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
            // Strip line comments and flatten each statement onto one line, but keep the interior of
            // dollar-quoted ($$...$$) blocks verbatim (e.g. a TOML index `options` that needs its
            // newlines). Splitting on `$$` alternates outside/inside (even = outside, odd = inside);
            // flatten only the outside. Files without `$$` are a single segment, unchanged.
            let query = query
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
                .to_owned();
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

/// Whether `query` reads its vector from the GUC, and so needs one bound before it can be planned.
///
/// This is about whether any vector is *available*, never about how many samples a query takes:
/// a reference plan reads the GUC just like a swept one, it simply does not iterate every vector.
fn reads_query_vector(query: &str) -> bool {
    query.contains(&format!("current_setting('{QVEC_GUC}')"))
}

/// Suffix given to a query file's second and later statements. Doubles as the dashboard's
/// chart-grouping separator, so alternatives plot alongside the statement they vary.
const ALTERNATIVE_MARKER: &str = " - alternative ";

/// Whether `query_type` names a non-first statement of its file.
fn is_alternative(query_type: &str) -> bool {
    query_type.contains(ALTERNATIVE_MARKER)
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
                format!("{query_type}{ALTERNATIVE_MARKER}{idx}")
            };
            (query_type, query)
        })
        .collect()
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
/// Bind one held-out vector for the next measured run. The query files read the vector through
/// `current_setting`, so the SQL text -- and therefore its `pg_stat_statements` queryid -- is
/// identical for every vector.
async fn set_query_vector(conn: &mut PgConnection, vector: &str) -> anyhow::Result<()> {
    sqlx::raw_sql(AssertSqlSafe(format!("SET {QVEC_GUC} = '{vector}';")))
        .execute(&mut *conn)
        .await
        .with_context(|| format!("Failed to set {QVEC_GUC}"))?;
    Ok(())
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

/// The held-out query vectors, when the dataset has them loaded (the recall fixtures a sweep
/// creates). Latency is then measured once per vector instead of by repeating a single one.
/// Datasets without them -- anything non-vector -- get an empty slice and the repeat-one path.
async fn load_query_vectors(conn: &mut PgConnection) -> Vec<String> {
    // An error here means no fixtures table: not a vector dataset, or recall was never loaded for
    // this run. Either way the caller falls back to repeating a single vector.
    sqlx::query_scalar::<_, String>("SELECT emb::text FROM cohere_queries ORDER BY id")
        .fetch_all(&mut *conn)
        .await
        .unwrap_or_default()
}

async fn execute_query_multiple_times(
    url: &str,
    query_type: &str,
    query: &str,
    sample_count: usize,
    fail_on_error: bool,
    query_vectors: &[String],
    sample_per_vector: bool,
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

    // Bind a vector before the EXPLAINs below: the kNN query files read it through
    // `current_setting`, and that errors if the GUC was never set. The measured passes rebind it
    // per vector, so which one seeds the plan does not affect any timing.
    match query_vectors.first() {
        Some(first) => set_query_vector(&mut conn, first).await?,
        // Nothing would bind it, so fail with the cause rather than an "unrecognized configuration
        // parameter" from deep inside the first EXPLAIN.
        None if reads_query_vector(query) => bail!(
            "`{query_type}` reads {QVEC_GUC} but no query vectors were loaded; the dataset's \
             recall fixtures must be present for a vector benchmark"
        ),
        None => {}
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
            "EXPLAIN (ANALYZE, VERBOSE, COSTS OFF, TIMING OFF, SUMMARY OFF) {measured_query}"
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

    // With a set of held-out query vectors, measure one timing per vector rather than repeating a
    // single vector: latency depends on which cells a vector routes to, so one vector reports the
    // cost of that vector, not of the workload. A first pass over every vector warms the cache for
    // all of them, then the measured pass records one sample each -- enough for percentiles.
    if sample_per_vector && !query_vectors.is_empty() {
        for (i, vector) in query_vectors.iter().enumerate() {
            set_query_vector(&mut conn, vector).await?;
            let warm = match run_once(&mut conn, query, stats_reset_query, &stats_query).await {
                Ok(v) => v,
                Err(err) => return on_query_error(query_type, err, fail_on_error),
            };
            if i == 0 {
                results.num_results = warm.1 as usize;
                results.cold = warm.0;
            }
        }
        results.per_vector = true;
        results.vector_hash = Some(fnv1a(query_vectors));
        for vector in query_vectors {
            set_query_vector(&mut conn, vector).await?;
            match run_once(&mut conn, query, stats_reset_query, &stats_query).await {
                Ok((time, _)) => results.samples.push(time),
                Err(err) => return on_query_error(query_type, err, fail_on_error),
            }
        }
        return Ok(Some(results));
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

    fn point(value: &str, recall: f64) -> SweepPoint {
        SweepPoint {
            value: value.to_owned(),
            recall,
            query: format!("SET eps={value}; SELECT 1"),
        }
    }

    #[test]
    fn selects_cheapest_point_reaching_each_target() {
        let points = vec![
            point("0.1", 0.88),
            point("0.2", 0.93),
            point("0.3", 0.96),
            point("0.5", 0.97),
            point("0.8", 0.995),
        ];
        let selected = select_points(&points);

        let by_label: HashMap<_, _> = selected.iter().map(|s| (s.label, s)).collect();
        // Cheapest at-or-above each target, not the nearest: 0.93 is closer to 0.90 than 0.88 is,
        // and 0.96 is the first to clear 0.95 even though 0.97 is also above it.
        assert_eq!(by_label["r90"].value, "0.2");
        assert_eq!(by_label["r95"].value, "0.3");
        assert_eq!(by_label["r99"].value, "0.8");
        assert!(selected.iter().all(|s| s.reached));
        // The selected point carries the query that produced it.
        assert_eq!(by_label["r95"].query, "SET eps=0.3; SELECT 1");
    }

    #[test]
    fn falls_back_to_best_point_and_flags_unreached_target() {
        let points = vec![point("0.1", 0.90), point("0.2", 0.94)];
        let selected = select_points(&points);
        let by_label: HashMap<_, _> = selected.iter().map(|s| (s.label, s)).collect();

        assert!(by_label["r90"].reached);
        // Nothing cleared 95% or 99%: report the best available, flagged so the caller can warn.
        for label in ["r95", "r99"] {
            assert!(!by_label[label].reached, "{label} should be unreached");
            assert_eq!(by_label[label].value, "0.2");
            assert_eq!(by_label[label].recall, 0.94);
        }
    }

    #[test]
    fn selection_ignores_the_order_values_were_measured_in() {
        let ascending = vec![point("0.1", 0.88), point("0.3", 0.96)];
        let descending = vec![point("0.3", 0.96), point("0.1", 0.88)];
        let pick = |pts: &[SweepPoint]| {
            select_points(pts)
                .into_iter()
                .map(|s| s.value)
                .collect::<Vec<_>>()
        };
        assert_eq!(pick(&ascending), pick(&descending));
    }

    #[test]
    fn empty_sweep_selects_nothing() {
        assert!(select_points(&[]).is_empty());
    }

    fn query_result(per_vector: bool, samples: Vec<f64>) -> QueryResult {
        QueryResult {
            query_type: "knn_top10_1pct@r95".to_owned(),
            query: "SELECT 1".to_owned(),
            results: QueryRunResults {
                cold: 11.108,
                samples,
                num_results: 10,
                per_vector,
                vector_hash: per_vector.then_some(0xdead_beef),
            },
        }
    }

    #[test]
    fn per_vector_results_publish_one_series_per_tracked_percentile() {
        let out = JSONBenchmarkResult::from_query_result(query_result(
            true,
            (1..=100).map(|i| i as f64).collect(),
        ));

        assert_eq!(
            out.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            [
                "knn_top10_1pct@r95 - p50",
                "knn_top10_1pct@r95 - p95",
                "knn_top10_1pct@r95 - p99"
            ]
        );
        assert!(
            out.iter()
                .all(|r| r.unit == "ms" && r.range.starts_with("95% CI ["))
        );
        // The raw samples ride on the p50 entry only, prefixed so the pairing metadata cannot
        // collide with the free-form query text at the end.
        assert!(
            out[0]
                .extra
                .starts_with("samples_fnv=00000000deadbeef; samples=[1.000,2.000,"),
            "{}",
            out[0].extra
        );
        assert!(out[0].extra.ends_with(
            "cold_query_ms=11.108; n=100; p50=50.500; p95=95.050; p99=99.010; query=SELECT 1"
        ));
        assert_eq!(
            out[1].extra,
            "cold_query_ms=11.108; n=100; p50=50.500; p95=95.050; p99=99.010; query=SELECT 1"
        );
    }

    /// Datasets that repeat one query have gh-pages history under the bare query name, and their
    /// samples are not a latency distribution. Renaming or re-valuing them would orphan that.
    #[test]
    fn repeated_query_results_still_publish_a_single_mean_series() {
        let out =
            JSONBenchmarkResult::from_query_result(query_result(false, vec![10.0, 12.0, 14.0]));

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "knn_top10_1pct@r95");
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

    /// pgvector access methods have no segments, so `extra` has nothing to carry.
    #[test]
    fn non_bm25_index_publishes_the_same_series_without_segments() {
        let series = JSONBenchmarkResult::from_index_creation(&IndexCreationResult::Other {
            duration_min_ms: 12.5,
            index_name: "cohere_hnsw_idx".to_owned(),
            index_size: 40960,
        });

        assert_eq!(
            series.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["cohere_hnsw_idx build time", "cohere_hnsw_idx index size"]
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

    /// The marker is produced in one place and matched in another; a rename that touched only one
    /// would silently restore the 100x sampling cost on reference plans.
    #[test]
    fn only_non_first_statements_count_as_alternatives() {
        let file_stem = "knn_top10_10pct";
        let named: Vec<String> = (0..3)
            .map(|idx| {
                if idx == 0 {
                    file_stem.to_owned()
                } else {
                    format!("{file_stem}{ALTERNATIVE_MARKER}{idx}")
                }
            })
            .collect();

        assert!(!is_alternative(&named[0]));
        assert!(named[1..].iter().all(|n| is_alternative(n)));
        // A swept operating point is still the file's first statement.
        assert!(!is_alternative(&format!("{file_stem}@r95")));
    }

    /// Passing an alternative an empty vector slice to stop it sampling per vector tripped this
    /// precondition and failed the whole run: the query still reads the GUC. Availability and
    /// sampling mode are separate concerns, and only the former belongs here.
    #[test]
    fn binding_precondition_tracks_availability_not_sampling_mode() {
        let knn = format!(
            "SELECT _id FROM t ORDER BY emb <=> current_setting('{QVEC_GUC}')::vector(1024) LIMIT 10"
        );
        assert!(reads_query_vector(&knn));
        assert!(!reads_query_vector("SELECT count(*) FROM t"));

        // Whether this is a swept point or a reference plan changes nothing about the precondition.
        for query_type in ["knn_top10_10pct@r95", "knn_top10_10pct - alternative 1"] {
            assert!(reads_query_vector(&knn), "{query_type}");
        }
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
}
