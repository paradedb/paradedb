use std::path::PathBuf;

use serde_json::json;

#[derive(Clone)]
pub enum TelemetryEvent {
    Deployment,
    DirectoryStatus { path: PathBuf, size: u64 },
}

impl TelemetryEvent {
    pub fn name(&self) -> String {
        match self {
            Self::Deployment { .. } => "Deployment".into(),
            Self::DirectoryStatus { .. } => "Directory Status".into(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Deployment => json!(serde_json::Value::Null),
            Self::DirectoryStatus { path, size } => json!({
                "path": path.to_str(),
                "size": size
            }),
        }
    }

    pub fn commit_sha(&self) -> Option<String> {
        std::env::var("COMMIT_SHA").ok()
    }
}
