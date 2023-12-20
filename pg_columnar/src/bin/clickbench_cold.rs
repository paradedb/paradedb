use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let current_dir = env::current_dir().expect("Failed to get current directory");

    // Function to recursively find the pg_columnar directory
    fn find_pg_columnar_dir(path: &Path) -> Option<PathBuf> {
        if path.ends_with("pg_columnar") {
            Some(path.to_path_buf())
        } else if path.ends_with("rd") {
            // If in 'rd/', then 'pg_columnar' should be a subdirectory
            let pg_columnar_path = path.join("pg_columnar");
            if pg_columnar_path.exists() {
                Some(pg_columnar_path)
            } else {
                None
            }
        } else if path.parent().is_some() {
            find_pg_columnar_dir(path.parent().unwrap())
        } else {
            None
        }
    }

    // Find the pg_columnar directory from the current directory
    let pg_columnar_dir =
        find_pg_columnar_dir(&current_dir).expect("Failed to find pg_columnar directory");

    // Construct the path to the script
    let script_path = pg_columnar_dir.join("benchmarks/clickbench/benchmark.sh");

    // Change directory if necessary
    if env::current_dir().unwrap() != pg_columnar_dir {
        env::set_current_dir(&pg_columnar_dir).expect("Failed to change directory");
    }

    // Run the script
    Command::new("sh")
        .arg(script_path)
        .arg("-t")
        .arg("pgrx")
        .status()
        .expect("Failed to execute benchmark script");
}
