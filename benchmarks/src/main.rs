use clap::Parser;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, value_parser = ["pg_search", "tuned_postgres"], default_value = "pg_search")]
    r#type: String,

    #[arg(long)]
    url: String,

    #[arg(long, default_value = "true")]
    prewarm: bool,

    #[arg(long, default_value = "10000000")]
    rows: u32,
}

fn main() {
    let args = Args::parse();
    let output_file = format!("results_{}.md", args.r#type);
    let mut file = File::create(&output_file).expect("Failed to create output file");

    write_benchmark_header(&mut file);
    write_test_info(&mut file, &args);
    write_postgres_settings(&mut file, &args.url);

    generate_test_data(&args.url, args.rows);
    process_index_creation(&mut file, &args.url, &args.r#type);
    run_benchmarks(&mut file, &args);
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

fn generate_test_data(url: &str, rows: u32) {
    let status = Command::new("psql")
        .arg(url)
        .arg("-v")
        .arg(format!("rows={}", rows))
        .arg("-f")
        .arg("generate.sql")
        .status()
        .expect("Failed to create table");

    if !status.success() {
        eprintln!("Failed to create table");
        std::process::exit(1);
    }
}

fn process_index_creation(file: &mut File, url: &str, r#type: &str) {
    writeln!(file, "\n## Index Creation Results").unwrap();
    writeln!(file, "| Index Name | Duration (min) | Index Size (MB) |").unwrap();
    writeln!(file, "|------------|----------------|-----------------|").unwrap();

    let index_sql = format!("create_index/{}.sql", r#type);
    let index_file = File::open(&index_sql).expect("Failed to open index file");
    let reader = BufReader::new(index_file);

    for line in reader.lines() {
        let statement = line.unwrap();
        if statement.trim().is_empty() {
            continue;
        }

        println!("{}", statement);

        let duration_min = execute_sql_with_timing(url, &statement);
        let index_name = extract_index_name(&statement);
        let index_size = get_index_size(url, index_name);

        writeln!(
            file,
            "| {} | {:.2} | {} |",
            index_name, duration_min, index_size
        )
        .unwrap();
    }
}

fn run_benchmarks(file: &mut File, args: &Args) {
    writeln!(file, "\n## Benchmark Results").unwrap();
    writeln!(
        file,
        "| Query Type | Run 1 (ms) | Run 2 (ms) | Run 3 (ms) | Rows Returned | Query |"
    )
    .unwrap();
    writeln!(
        file,
        "|------------|------------|------------|------------|---------------|--------|"
    )
    .unwrap();

    if args.prewarm {
        prewarm_indexes(&args.url, &args.r#type);
    }

    let queries_dir = format!("queries/{}", args.r#type);
    for entry in std::fs::read_dir(queries_dir).expect("Failed to read queries directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) != Some("sql") {
            continue;
        }

        let query_type = path.file_stem().unwrap().to_string_lossy();
        let query_content = std::fs::read_to_string(&path).expect("Failed to read query file");

        for query in query_content.split(';') {
            let query = query.trim();
            if query.is_empty() {
                continue;
            }

            println!("Query Type: {}\nQuery: {}", query_type, query);

            let (results, num_results) = execute_query_multiple_times(&args.url, query, 3);

            let md_query = query.replace("|", "\\|");
            writeln!(
                file,
                "| {} | {:.0} | {:.0} | {:.0} | {} | `{}` |",
                query_type, results[0], results[1], results[2], num_results, md_query
            )
            .unwrap();

            println!(
                "Run 1: {:.0}ms | Run 2: {:.0}ms | Run 3: {:.0}ms | Results: {}\n",
                results[0], results[1], results[2], num_results
            );
        }
    }
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
        .unwrap()
}

fn prewarm_indexes(url: &str, r#type: &str) {
    let prewarm_sql = format!("prewarm/{}.sql", r#type);
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
