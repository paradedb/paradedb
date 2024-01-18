use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let root_dir = env::current_dir().expect("Failed to get current directory");
    let benchmark_dir = root_dir.join("benchmarks/clickbench/benchmark.sh");

    if !Path::new(&benchmark_dir).exists() {
        panic!("Benchmark script not found. Please run from the pg_analytics/ directory.");
    }

    Command::new("sh")
        .arg(benchmark_dir)
        .arg("-t")
        .arg("pgrx")
        .status()
        .expect("Failed to execute benchmark script");
}
