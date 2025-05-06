// Copyright (c) 2023-2025 ParadeDB, Inc.
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

// This script generates a regression tests file for all SQL files in the tests/sql directory.
// Run it with `cargo run --bin generate_regression_tests`

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    // Get the workspace directory
    let workspace_dir = env::current_dir().expect("Failed to get current directory");

    // Look for SQL files in the sql directory relative to the current directory
    let sql_dir = workspace_dir.join("tests").join("sql");

    println!("Looking for SQL files in: {}", sql_dir.display());

    // Find all SQL files
    let sql_files = find_sql_files(&sql_dir);

    if sql_files.is_empty() {
        println!(
            "No SQL files found in {}. Make sure your directory structure is correct.",
            sql_dir.display()
        );
        return;
    }

    // Print found SQL files
    println!("Found SQL files:");
    for sql_file in &sql_files {
        println!("  {}", sql_file.display());
    }

    // Generate test file content
    let mut test_file_content = generate_test_file_header();

    // Generate a test function for each SQL file
    for sql_file in &sql_files {
        let file_stem = sql_file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let test_name = format!("test_{}", file_stem.replace("-", "_"));
        let relative_path = sql_file.strip_prefix(&workspace_dir).unwrap_or(sql_file);

        test_file_content.push_str(&generate_test_function(&test_name, relative_path));
    }

    // Close the module
    test_file_content.push_str("}\n");

    // Create the tests directory if it doesn't exist
    let tests_dir = workspace_dir.join("tests").join("tests");
    if !tests_dir.exists() {
        fs::create_dir_all(&tests_dir).expect("Failed to create tests directory");
    }

    // Write the test file
    let output_path = tests_dir.join("regression_tests.rs");
    let mut file = File::create(&output_path).expect("Failed to create output file");
    file.write_all(test_file_content.as_bytes())
        .expect("Failed to write to output file");

    println!(
        "Generated regression tests with {} test functions in {}",
        sql_files.len(),
        output_path.display()
    );
    println!("\nTo run these tests:");
    println!("  cargo test --test regression_tests");
    println!("\nTo run a specific test:");
    println!("  cargo test --test regression_tests test_<name>");
    println!("\nTo regenerate expected outputs for a test:");
    println!(
        "  REGENERATE_EXPECTED=1 cargo test --test regression_tests test_<name> -- --nocapture"
    );
}

/// Finds all SQL files in the specified directory
fn find_sql_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();

    println!("Searching for SQL files in: {}", dir.display());

    if !dir.exists() {
        println!("Directory does not exist: {}", dir.display());
        return result;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively search in subdirectories, but skip the 'expected' directory
                if let Some(dir_name) = path.file_name() {
                    if dir_name != "expected" {
                        result.extend(find_sql_files(&path));
                    }
                }
            } else if let Some(extension) = path.extension() {
                // Only add .sql files
                if extension == "sql" {
                    println!("Found SQL file: {}", path.display());
                    result.push(path);
                }
            }
        }
    }

    result
}

/// Generates the header for the test file
fn generate_test_file_header() -> String {
    r#"// Copyright (c) 2023-2025 ParadeDB, Inc.
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

//! This file is auto-generated by the generate_regression_tests.rs script.
//! DO NOT EDIT MANUALLY - Changes will be overwritten when the script is run again.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Helper functions for regression testing
fn run_sql_file(sql_file: &Path) -> String {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    // Execute the SQL file using psql with echo all flag
    let output = Command::new("psql")
        .arg(database_url)
        .arg("-a") // Echo all output, including SQL commands
        .arg("-f")
        .arg(sql_file)
        .output()
        .expect("Failed to execute SQL file");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn normalize_output(sql_file: &Path, output: &str) -> Result<String, String> {
    let sql_file_content =
        std::fs::read_to_string(sql_file).map_err(|e| format!("Failed to read SQL file: {}", e))?;
    let sql_file_first_line = sql_file_content.lines().next();
    // Find where the SQL content starts - typically after setup messages
    let lines: Vec<&str> = output.lines().collect();
    let mut sql_start_idx = 0;

    // First, look for the actual SQL file content by finding SQL comments or commands
    // that match what's in the original SQL file
    if let Some(first_line) = sql_file_first_line {
        sql_start_idx = lines
            .iter()
            .position(|line| line.trim() == first_line.trim())
            .unwrap_or(0);
    }

    // Extract only relevant SQL output
    Ok(lines[sql_start_idx..]
        .iter()
        .filter(|line| !line.contains("Time:")) // Remove timing information
        .filter(|line| !line.starts_with("++") && !line.starts_with("+")) // Remove command execution prefixes
        .filter(|line| !line.contains("cargo pgrx")) // Remove pgrx related commands
        .filter(|line| !line.starts_with("psql:")) // Remove psql file path prefixes
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join("\n"))
}

fn compare_outputs(sql_file: &Path, actual: &str, expected: &str) -> Result<(), String> {
    let normalized_actual = normalize_output(sql_file, actual)?;
    let normalized_expected = normalize_output(sql_file, expected)?;

    if normalized_actual == normalized_expected {
        Ok(())
    } else {
        // Find the first line that differs for better error reporting
        let actual_lines: Vec<&str> = normalized_actual.lines().collect();
        let expected_lines: Vec<&str> = normalized_expected.lines().collect();

        let mut diff_line = 0;
        let min_lines = actual_lines.len().min(expected_lines.len());

        for i in 0..min_lines {
            if actual_lines[i] != expected_lines[i] {
                diff_line = i + 1;
                break;
            }
        }

        if diff_line == 0 && actual_lines.len() != expected_lines.len() {
            diff_line = min_lines + 1;
        }

        Err(format!(
            "Output mismatch at line {}:\nExpected ({} lines):\n{}\n\nActual ({} lines):\n{}",
            diff_line,
            expected_lines.len(),
            normalized_expected,
            actual_lines.len(),
            normalized_actual
        ))
    }
}

fn get_expected_output_path(sql_file: &Path) -> PathBuf {
    let parent = sql_file.parent().unwrap_or(Path::new(""));
    let file_stem = sql_file.file_stem().unwrap_or_default();

    parent
        .join("expected")
        .join(format!("{}.out", file_stem.to_string_lossy()))
}

fn create_expected_output(sql_file: &Path) -> Result<(), String> {
    let output = run_sql_file(sql_file);
    let expected_path = get_expected_output_path(sql_file);

    // Make sure the expected directory exists
    if let Some(parent) = expected_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create expected directory: {}", e))?;
        }
    }

    // Normalize the output before saving
    let normalized_output = normalize_output(sql_file, &output)?;

    // Write the normalized output to the expected file
    fs::write(&expected_path, normalized_output)
        .map_err(|e| format!("Failed to write expected output file: {}", e))?;

    println!(
        "Created expected output file at {}",
        expected_path.display()
    );
    Ok(())
}

fn run_regression_test(sql_file: &Path, regenerate: bool) -> Result<(), String> {
    let file_stem = sql_file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    println!("Running regression test: {}", file_stem);

    // Get the expected output path
    let expected_path = get_expected_output_path(sql_file);

    // If regenerate is true or the expected output doesn't exist, create it
    if regenerate || !expected_path.exists() {
        if expected_path.exists() && regenerate {
            println!("Regenerating expected output for {}", file_stem);
        } else {
            println!("Creating expected output for {}", file_stem);
        }
        create_expected_output(sql_file)?;
        return Ok(());
    }

    // Read expected output
    let expected_output = fs::read_to_string(&expected_path)
        .map_err(|e| format!("Failed to read expected output: {}", e))?;

    // Run SQL file
    let actual_output = run_sql_file(sql_file);

    // Compare outputs
    compare_outputs(&sql_file, &actual_output, &expected_output)
}

// Macro to define a SQL regression test
macro_rules! sql_regression_test {
    ($name:ident, $sql_file:expr) => {
        #[test]
        fn $name() {
            let workspace_dir = env::current_dir().expect("Failed to get current directory");
            let sql_file = workspace_dir.join($sql_file);

            // Check if we should regenerate expected output
            let regenerate = env::var("REGENERATE_EXPECTED").unwrap_or_default() == "1";

            match run_regression_test(&sql_file, regenerate) {
                Ok(()) => println!("✅ Test passed: {}", stringify!($name)),
                Err(err) => {
                    eprintln!("❌ Test failed: {}\n{}\n", stringify!($name), err);

                    // Add helpful instructions for updating the expected output
                    eprintln!("\nTo update the expected output, run:");
                    eprintln!("    REGENERATE_EXPECTED=1 cargo test --test regression_tests {} -- --nocapture", stringify!($name));

                    panic!("SQL regression test for {} failed: {}", stringify!($name), err);
                }
            }
        }
    };
}

// Individual test functions for each SQL file
#[cfg(test)]
mod tests {
    use super::*;

"#.to_string()
}

/// Generates a test function for a SQL file
fn generate_test_function(test_name: &str, relative_path: &Path) -> String {
    format!(
        r#"    sql_regression_test!({test_name}, "{path}");
"#,
        test_name = test_name,
        path = Path::new("..")
            .join(relative_path)
            .to_string_lossy()
            .replace('\\', "/")
    )
}
