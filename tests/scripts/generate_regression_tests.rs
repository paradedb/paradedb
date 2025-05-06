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

// This script updates the regression_tests.rs file with test functions for all SQL files
// in the tests/sql directory.

use std::env;
use std::fs;
use std::io::Read;
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

    // Generate test function declarations for each SQL file
    let test_declarations = generate_test_declarations(&sql_files, &workspace_dir);

    // Path to the regression_tests.rs file
    let regression_tests_path = workspace_dir
        .join("tests")
        .join("tests")
        .join("regression_tests.rs");

    // Update the regression_tests.rs file with the generated test functions
    update_regression_tests_file(&regression_tests_path, &test_declarations);

    println!(
        "Updated regression tests with {} test functions in {}",
        sql_files.len(),
        regression_tests_path.display()
    );
}

/// Finds all SQL files in the specified directory
fn find_sql_files(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if !dir.exists() {
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
                    result.push(path);
                }
            }
        }
    }

    result
}

/// Generates test function declarations for each SQL file
fn generate_test_declarations(sql_files: &[PathBuf], workspace_dir: &Path) -> String {
    let mut declarations = String::new();
    declarations.push_str("    // Run 'cargo run --bin generate_regression_tests' to update\n\n");

    for sql_file in sql_files {
        // Get the relative path from the workspace dir to the SQL file
        let rel_path = Path::new("../")
            .join(sql_file.strip_prefix(workspace_dir).unwrap_or(sql_file))
            .to_string_lossy()
            .replace('\\', "/");

        // Generate the test name from the file name
        let file_stem = sql_file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let test_name = format!("test_{}", file_stem.replace('-', "_"));

        // Generate the test declaration
        declarations.push_str(&format!(
            "    sql_regression_test!({}, \"{}\");\n",
            test_name, rel_path
        ));
    }

    declarations
}

/// Updates the regression_tests.rs file with the generated test functions
fn update_regression_tests_file(file_path: &Path, test_declarations: &str) {
    // Read the file content
    let mut file_content = String::new();
    fs::File::open(file_path)
        .and_then(|mut file| file.read_to_string(&mut file_content))
        .expect("Failed to read regression_tests.rs");

    // Find the start and end markers for the test functions
    let start_marker = "    // AUTO-GENERATED TEST FUNCTIONS - DO NOT MODIFY MANUALLY";
    let end_marker = "    // END AUTO-GENERATED TEST FUNCTIONS";

    // Split the file content at the start marker
    let parts: Vec<&str> = file_content.split(start_marker).collect();
    if parts.len() < 2 {
        eprintln!("Could not find the start marker in regression_tests.rs");
        return;
    }

    let before_start = parts[0];
    let after_start = parts[1];

    // Split the content after the start marker at the end marker
    let parts: Vec<&str> = after_start.split(end_marker).collect();
    if parts.len() < 2 {
        eprintln!("Could not find the end marker in regression_tests.rs");
        return;
    }

    let after_end = parts[1];

    // Create the updated content with the new test declarations
    let updated_content = format!(
        "{}{}\n{}\n{}{}",
        before_start, start_marker, test_declarations, end_marker, after_end
    );

    // Write the updated content back to the file
    fs::write(file_path, updated_content).expect("Failed to write to regression_tests.rs");
}
