use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Set default to "hot" if no argument is provided
    let benchmark_mode = if args.len() > 1 { &args[1] } else { "hot" };

    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Function to recursively find the pg_analytics directory
    fn find_pg_analytics_dir(path: &Path) -> Option<PathBuf> {
        if path.ends_with("pg_analytics") {
            Some(path.to_path_buf())
        } else if path.ends_with("rd") {
            let pg_analytics_path = path.join("pg_analytics");
            if pg_analytics_path.exists() {
                Some(pg_analytics_path)
            } else {
                None
            }
        } else if let Some(parent) = path.parent() {
            find_pg_analytics_dir(parent)
        } else {
            None
        }
    }

    let pg_analytics_dir =
        find_pg_analytics_dir(&current_dir).expect("Failed to find pg_analytics directory");

    let script_path = pg_analytics_dir.join("benchmarks/clickbench/benchmark.sh");

    if current_dir != pg_analytics_dir {
        env::set_current_dir(&pg_analytics_dir).expect("Failed to change directory");
    }

    Command::new("sh")
        .arg(script_path)
        .arg("-t")
        .arg("pgrx")
        .arg("-s")
        .arg(benchmark_mode)
        .status()
        .expect("Failed to execute benchmark script");
}
