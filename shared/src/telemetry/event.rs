use std::path::PathBuf;

use serde_json::json;

#[derive(Clone)]
pub enum TelemetryEvent {
    Deployment {
        extension: String,
    },
    DirectoryStatus {
        extension: String,
        path: PathBuf,
        size: u64,
    },
}

impl TelemetryEvent {
    pub fn name(&self) -> String {
        match self {
            Self::Deployment { extension } => format!("{extension} Deployment"),
            Self::DirectoryStatus { extension, .. } => format!("{extension} Directory Status"),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Deployment { .. } => json!(serde_json::Value::Null),
            Self::DirectoryStatus { path, size, .. } => json!({
                "path": path.to_str(),
                "size": size
            }),
        }
    }

    pub fn commit_sha(&self) -> Option<String> {
        option_env!("COMMIT_SHA").map(String::from)
    }

    pub fn enabled(&self) -> bool {
        option_env!("PARADEDB_TELEMETRY")
            .map(|s| s.trim().to_lowercase() == "true")
            .unwrap_or(cfg!(telemetry))
    }
}
