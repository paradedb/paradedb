use std::{path::PathBuf, time::SystemTime};

use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum TelemetryEvent {
    Deployment {
        timestamp: SystemTime,
        arch: String,
        extension_name: String,
        extension_version: String,
        os_type: String,
        os_version: String,
        postgres_version: String,
    },
    DirectoryStatus {
        extension_name: String,
        path: PathBuf,
        size: u64,
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
