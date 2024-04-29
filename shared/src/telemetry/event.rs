use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TelemetryEvent {
    Deployment {
        timestamp: String,
        arch: String,
        extension_name: String,
        extension_version: String,
        extension_path: PathBuf,
        os_type: String,
        os_version: String,
        replication_mode: Option<String>,
        postgres_version: String,
        postgres_version_details: String,
    },
    DirectoryStatus {
        extension_name: String,
        replication_mode: Option<String>,
        path: PathBuf,
        size: u64,
        humansize: String,
    },
}

impl TelemetryEvent {
    pub fn name(&self) -> String {
        match self {
            Self::Deployment { extension_name, .. } => format!("{extension_name} Deployment"),
            Self::DirectoryStatus { extension_name, .. } => {
                format!("{extension_name} Directory Status")
            }
        }
    }

    pub fn commit_sha(&self) -> Option<String> {
        option_env!("COMMIT_SHA").map(String::from)
    }
}
