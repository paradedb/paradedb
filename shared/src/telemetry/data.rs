use serde_json::{json, Value};
use std::fs;

pub fn read_telemetry_data(directory_path: &str) -> Result<Value, String> {
    // Get the metadata for the specified directory
    let dir_metadata = fs::metadata(directory_path)
        .map_err(|e| format!("Error getting metadata for {}: {}", directory_path, e))?;

    // Check if the specified path is a directory
    if !dir_metadata.is_dir() {
        return Err(format!("{} is not a directory", directory_path));
    }

    // Read the size of the directory
    let dir_size = dir_metadata.len();

    // Create a JSON object with the directory size
    let json_data = json!({
        "directory": directory_path,
        "size": dir_size,
    });

    Ok(json_data)
}
