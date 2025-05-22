use clap::Parser;
use paradedb::micro_benchmarks::benchmark_mixed_fast_fields;
use sqlx::{Connection, PgConnection};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_parser = ["pg_search", "tuned_postgres"], default_value = "pg_search")]
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
    #[arg(long, value_parser = ["md", "csv"], default_value = "md")]
    output: String,

    #[arg(long, value_parser = ["fastfields", "sql"], default_value = "sql")]
    benchmark: String,

    #[arg(long, default_value = "2")]
    warmups: usize,

    #[arg(long, default_value = "5")]
    iterations: usize,

    #[arg(long, default_value = "100000")]
    batch_size: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.benchmark == "fastfields" {
        let mut conn = PgConnection::connect(&args.url).await.unwrap();
        let res = benchmark_mixed_fast_fields(
            &mut conn,
            args.runs,
            args.warmups,
            args.rows as usize,
            args.batch_size,
        )
        .await;
        println!("Mixed Fast Fields Benchmark Completed: {:?}", res);
    } else if args.benchmark == "sql" {

        if !args.existing {
            generate_test_data(&args.url, &args.dataset, args.rows);
        }

        match args.output.as_str() {
            "md" => generate_markdown_output(&args),
            "csv" => generate_csv_output(&args),
            _ => unreachable!("Clap ensures only md or csv are valid"),
        }
    } else {
        eprintln!("Invalid benchmark type");
        std::process::exit(1);
    }
}

fn generate_markdown_output(args: &Args) {
    let output_file = format!("results_{}.md", args.r#type);
    let mut file = File::create(&output_file).expect("Failed to create output file");

    write_benchmark_header(&mut file);
    write_test_info(&mut file, args);
    write_postgres_settings(&mut file, &args.url);
    if !args.existing {
        process_index_creation(&mut file, &args.url, &args.dataset, &args.r#type);
    }
    run_benchmarks_md(&mut file, args);
}

fn generate_csv_output(args: &Args) {
    write_test_info_csv(args);
    write_postgres_settings_csv(&args.url, &args.r#type);
    if !args.existing {
        process_index_creation_csv(&args.url, &args.dataset, &args.r#type);
    }
    run_benchmarks_csv(args);
}

fn write_test_info_csv(args: &Args) {
    let filename = format!("results_{}_test_info.csv", args.r#type);
    let mut file = File::create(&filename).expect("Failed to create test info CSV");

    writeln!(file, "Key,Value").unwrap();
    writeln!(file, "Number of Rows,{}", args.rows).unwrap();
    writeln!(file, "Test Type,{}", args.r#type).unwrap();
    writeln!(file, "Prewarm,{}", args.prewarm).unwrap();
    writeln!(file, "Vacuum,{}", args.vacuum).unwrap();

    if args.r#type == "pg_search" {
        if let Ok(output) = execute_psql_command(
            &args.url,
            "SELECT version, githash, build_mode FROM paradedb.version_info();",
        ) {
            let parts: Vec<&str> = output.trim().split('|').collect();
            if parts.len() == 3 {
                writeln!(file, "pg_search Version,{}", parts[0].trim()).unwrap();
                writeln!(file, "pg_search Git Hash,{}", parts[1].trim()).unwrap();
                writeln!(file, "pg_search Build Mode,{}", parts[2].trim()).unwrap();
            }
        }
    }
}

fn write_postgres_settings_csv(url: &str, test_type: &str) {
    let filename = format!("results_{}_postgres_settings.csv", test_type);
    let mut file = File::create(&filename).expect("Failed to create postgres settings CSV");

    writeln!(file, "Setting,Value").unwrap();

    let settings = vec![
        "maintenance_work_mem",
        "shared_buffers",
        "max_parallel_workers",
        "max_worker_processes",
        "max_parallel_workers_per_gather",
    ];

    for setting in settings {
        let value = execute_psql_command(url, &format!("SHOW {};", setting))
            .expect("Failed to get postgres setting")
            .trim()
            .to_string();
        writeln!(file, "{},{}", setting, value).unwrap();
    }
}

fn process_index_creation_csv(url: &str, dataset: &str, test_type: &str) {
    let filename = format!("results_{test_type}_index_creation.csv");
    let mut file = File::create(&filename).expect("Failed to create index creation CSV");

    writeln!(
        file,
        "Index Name,Duration (min),Index Size (MB),Segment Count"
    )
    .unwrap();

    let index_sql = format!("datasets/{dataset}/create_index/{test_type}.sql");
    for statement in queries(Path::new(&index_sql)) {
        println!("{}", statement);

        let duration_min = execute_sql_with_timing(url, &statement);
        let index_name = extract_index_name(&statement);
        let index_size = get_index_size(url, index_name);
        let segment_count = get_segment_count(url, index_name);

        writeln!(
            file,
            "{},{:.2},{},{}",
            index_name, duration_min, index_size, segment_count
        )
        .unwrap();
    }
}

fn run_benchmarks_csv(args: &Args) {
    let filename = format!("results_{}_benchmark_results.csv", args.r#type);
    let mut file = File::create(&filename).expect("Failed to create benchmark results CSV");

    // Write header
    let mut header = String::from("Query Type");
    for i in 1..=args.runs {
        header.push_str(&format!(",Run {} (ms)", i));
    }
    header.push_str(",Rows Returned,Query");
    writeln!(file, "{}", header).unwrap();

    if args.vacuum {
        execute_psql_command(&args.url, "VACUUM ANALYZE;").expect("Failed to vacuum");
    }

    if args.prewarm {
        prewarm_indexes(&args.url, &args.dataset, &args.r#type);
    }

    let queries_dir = format!("datasets/{}/queries/{}", args.dataset, args.r#type);
    for entry in std::fs::read_dir(queries_dir).expect("Failed to read queries directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }

        let query_type = path.file_stem().unwrap().to_string_lossy();
        for query in queries(&path) {
            println!("Query Type: {}\nQuery: {}", query_type, query);

            let (results, num_results) = execute_query_multiple_times(&args.url, &query, args.runs);

            let mut result_line = format!("{}", query_type);
            for &result in &results {
                result_line.push_str(&format!(",{:.0}", result));
            }
            result_line.push_str(&format!(
                ",{},\"{}\"",
                num_results,
                query.replace("\"", "\"\"")
            ));
            writeln!(file, "{}", result_line).unwrap();

            println!("Results: {:?} | Rows Returned: {}\n", results, num_results);
        }
    }
}

fn write_benchmark_header(file: &mut File) {
    writeln!(file, "# Benchmark Results").unwrap();
}

fn write_test_info(file: &mut File, args: &Args) {
    writeln!(file, "\n## Test Info").unwrap();
    writeln!(file, "| Key         | Value       |").unwrap();
    writeln!(file, "|-------------|-------------|").unwrap();
    writeln!(file, "| Number of Rows | {} |", args.rows).unwrap();
    writeln!(file, "| Test Type   | {} |", args.r#type).unwrap();
    writeln!(file, "| Prewarm     | {} |", args.prewarm).unwrap();
    writeln!(file, "| Vacuum      | {} |", args.vacuum).unwrap();

    if args.r#type == "pg_search" {
        if let Ok(output) = execute_psql_command(
            &args.url,
            "SELECT version, githash, build_mode FROM paradedb.version_info();",
        ) {
            let parts: Vec<&str> = output.trim().split('|').collect();
            if parts.len() == 3 {
                writeln!(file, "| pg_search Version | {} |", parts[0].trim()).unwrap();
                writeln!(file, "| pg_search Git Hash | {} |", parts[1].trim()).unwrap();
                writeln!(file, "| pg_search Build Mode | {} |", parts[2].trim()).unwrap();
            }
        }
    }
}

fn write_postgres_settings(file: &mut File, url: &str) {
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

    for setting in settings {
        let value = execute_psql_command(url, &format!("SHOW {};", setting))
            .expect("Failed to get postgres setting")
            .trim()
            .to_string();
        writeln!(file, "| {} | {} |", setting, value).unwrap();
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

fn process_index_creation(file: &mut File, url: &str, dataset: &str, r#type: &str) {
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

    let index_sql = format!("datasets/{dataset}/create_index/{}.sql", r#type);
    for statement in queries(Path::new(&index_sql)) {
        println!("{}", statement);

        let duration_min = execute_sql_with_timing(url, &statement);
        let index_name = extract_index_name(&statement);
        let index_size = get_index_size(url, index_name);
        let segment_count = get_segment_count(url, index_name);

        writeln!(
            file,
            "| {} | {:.2} | {} | {} |",
            index_name, duration_min, index_size, segment_count
        )
        .unwrap();
    }
}

fn run_benchmarks_md(file: &mut File, args: &Args) {
    writeln!(file, "\n## Benchmark Results").unwrap();

    write_benchmark_table_header(file, args.runs);

    if args.vacuum {
        execute_psql_command(&args.url, "VACUUM ANALYZE benchmark_logs;")
            .expect("Failed to vacuum");
    }

    if args.prewarm {
        prewarm_indexes(&args.url, &args.dataset, &args.r#type);
    }

    let queries_dir = format!("datasets/{}/queries/{}", args.dataset, args.r#type);
    for entry in std::fs::read_dir(queries_dir).expect("Failed to read queries directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }

        let query_type = path.file_stem().unwrap().to_string_lossy();
        for query in queries(&path) {
            println!("Query Type: {}\nQuery: {}", query_type, query);

            let (results, num_results) = execute_query_multiple_times(&args.url, &query, args.runs);

            let md_query = query.replace("|", "\\|");
            write_benchmark_results(file, &query_type, &results, num_results, &md_query);

            println!("Results: {:?} | Rows Returned: {}\n", results, num_results);
        }
    }
}

fn write_benchmark_table_header(file: &mut File, runs: usize) {
    let mut header = String::from("| Query Type ");
    let mut separator = String::from("|------------");

    for i in 1..=runs {
        header.push_str(&format!("| Run {} (ms) ", i));
        separator.push_str("|------------");
    }

    header.push_str("| Rows Returned | Query |");
    separator.push_str("|---------------|--------|");

    writeln!(file, "{}", header).unwrap();
    writeln!(file, "{}", separator).unwrap();
}

fn write_benchmark_results(
    file: &mut File,
    query_type: &str,
    results: &[f64],
    num_results: usize,
    md_query: &str,
) {
    let mut result_line = format!("| {} ", query_type);

    for &result in results {
        result_line.push_str(&format!("| {:.0} ", result));
    }

    result_line.push_str(&format!("| {} | `{}` |", num_results, md_query));
    writeln!(file, "{}", result_line).unwrap();
}

///
/// Return an iterator over the query strings contained in the given file path.
///
/// Strips comments and flattens each query onto a single line.
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
                .join(" ");
            if query.is_empty() {
                None
            } else {
                Some(query)
            }
        })
        .collect()
}

fn execute_psql_command(url: &str, command: &str) -> Result<String, std::io::Error> {
    let output = Command::new("psql")
        .arg(url)
        .arg("-t")
        .arg("-c")
        .arg(command)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn execute_sql_with_timing(url: &str, statement: &str) -> f64 {
    let output = Command::new("psql")
        .arg(url)
        .arg("-t")
        .arg("-c")
        .arg("\\timing")
        .arg("-c")
        .arg(statement)
        .output()
        .expect("Failed to execute SQL");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("psql command failed: {} \nError: {}", output.status, stderr);
    }

    let timing = String::from_utf8_lossy(&output.stdout);
    let duration_ms = timing
        .lines()
        .find(|line| line.contains("Time"))
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("Failed to parse timing")
        .parse::<f64>()
        .unwrap();

    duration_ms / (1000.0 * 60.0)
}

fn extract_index_name(statement: &str) -> &str {
    statement
        .split_whitespace()
        .nth(2)
        .expect("Failed to parse index name")
}

fn get_index_size(url: &str, index_name: &str) -> i64 {
    let size_query = format!("SELECT pg_relation_size('{}') / (1024 * 1024);", index_name);
    let output = Command::new("psql")
        .arg(url)
        .arg("-t")
        .arg("-c")
        .arg(&size_query)
        .output()
        .expect("Failed to get index size");

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i64>()
        .expect("Failed to get index size")
}

fn get_segment_count(url: &str, index_name: &str) -> i64 {
    let query = format!(
        "SELECT count(*) FROM paradedb.index_info('{}');",
        index_name
    );
    let output = Command::new("psql")
        .arg(url)
        .arg("-t")
        .arg("-c")
        .arg(&query)
        .output()
        .expect("Failed to get segment count");

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i64>()
        .expect("Failed to parse segment count")
}

fn prewarm_indexes(url: &str, dataset: &str, r#type: &str) {
    let prewarm_sql = format!("datasets/{dataset}/prewarm/{}.sql", r#type);
    let status = Command::new("psql")
        .arg(url)
        .arg("-f")
        .arg(&prewarm_sql)
        .status()
        .expect("Failed to prewarm indexes");

    if !status.success() {
        eprintln!("Failed to prewarm indexes");
        std::process::exit(1);
    }
}

fn execute_query_multiple_times(url: &str, query: &str, times: usize) -> (Vec<f64>, usize) {
    let mut results = Vec::new();
    let mut num_results = 0;

    for i in 0..times {
        let output = Command::new("psql")
            .arg(url)
            .arg("-t")
            .arg("-c")
            .arg("\\timing")
            .arg("-c")
            .arg(query)
            .output()
            .expect("Failed to execute query");

        let output_str = String::from_utf8_lossy(&output.stdout);
        let duration = output_str
            .lines()
            .find(|line| line.contains("Time"))
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("Failed to parse timing")
            .parse::<f64>()
            .unwrap();

        results.push(duration);

        if i == 0 {
            num_results = output_str
                .lines()
                .filter(|line| !line.contains("Time") && !line.trim().is_empty())
                .count()
                - 1;
        }
    }

    (results, num_results)
}
