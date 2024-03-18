use serde_json::json; // Ensure you have `serde_json` in your Cargo.toml
use serde_json::Value;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::TelemetryError;

pub struct Directory;

impl Directory {
    pub fn postgres() -> Result<PathBuf, TelemetryError> {
        std::env::var("PGDATA")
            .map(PathBuf::from)
            .map_err(TelemetryError::NoPgData)
    }

    pub fn extension(extension_name: &str) -> Result<PathBuf, TelemetryError> {
        Ok(Self::postgres()?.join(match extension_name {
            "pg_bm25" => "paradedb",
            "pg_analytics" => "deltalake",
            _ => return Err(TelemetryError::UnknownExtension(extension_name.to_string())),
        }))
    }
}

pub fn read_telemetry_data(extension_name: &str) -> Result<Value, TelemetryError> {
    // Customize the path based on the extension name
    let directory_path = Directory::extension(extension_name)?;
    let dir_size = WalkDir::new(&directory_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .fold(0, |acc, m| acc + m.len());

    // Create a JSON object with the directory size
    let json_data = json!({
        "directory": directory_path.to_str(),
        "size": dir_size,
    });

    Ok(json_data)
}
