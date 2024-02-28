use serde_json::json; // Ensure you have `serde_json` in your Cargo.toml
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

// Function to get the PostgreSQL data directory from the PGDATA environment variable
pub fn get_postgres_data_directory() -> Option<String> {
    env::var("PGDATA").ok()
}

pub fn read_telemetry_data(extension_name: String) -> Result<Value, String> {
    // Determine the base PostgreSQL data directory
    let pg_data_directory =
        get_postgres_data_directory().ok_or("PGDATA environment variable is not set")?;

    // Customize the path based on the extension name
    let directory_path = PathBuf::from(pg_data_directory).join(match extension_name.as_str() {
        "pg_bm25" => "paradedb",
        "pg_analytics" => "deltalake",
        _ => return Err("Unknown extension name".to_string()),
    });

    // Convert the PathBuf back to a String for use in error messages and JSON object
    let directory_path_str = directory_path
        .to_str()
        .ok_or("Failed to convert directory path to string")?;

    // Get the metadata for the specified directory
    let dir_metadata = fs::metadata(&directory_path)
        .map_err(|e| format!("Error getting metadata for {}: {}", directory_path_str, e))?;

    // Check if the specified path is a directory
    if !dir_metadata.is_dir() {
        return Err(format!("{} is not a directory", directory_path_str));
    }

    // Calculate total size of files in the directory
    let dir_size: u64 = fs::read_dir(&directory_path)
        .map_err(|e| format!("Error reading directory {}: {}", directory_path_str, e))?
        .filter_map(Result::ok) // Filter out any Err results during iteration
        .map(|entry| entry.path())
        .filter_map(|path| fs::metadata(path).ok()) // Ignore errors in metadata retrieval
        .filter(|metadata| metadata.is_file()) // Consider only files for size calculation
        .map(|metadata| metadata.len()) // Extract file size
        .sum(); // Explicitly telling the compiler we are summing u64 values

    // Create a JSON object with the directory size
    let json_data = json!({
        "directory": directory_path_str,
        "size": dir_size,
    });

    Ok(json_data)
}
