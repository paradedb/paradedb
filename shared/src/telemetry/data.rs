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

pub trait DirSize {
    fn dir_size(&self) -> u64;
}

impl DirSize for PathBuf {
    fn dir_size(&self) -> u64 {
        WalkDir::new(&self)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len())
    }
}
