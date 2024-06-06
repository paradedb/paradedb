use super::{DirectoryStore, TelemetryConfigStore};
use anyhow::{anyhow, Result};
use std::{fs, path::PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct PostgresDirectoryStore {
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl DirectoryStore for PostgresDirectoryStore {
    fn root_path(&self) -> Result<PathBuf> {
        self.config_store.root_data_directory()
    }

    fn extension_path(&self) -> Result<PathBuf> {
        let root = self.root_path()?;
        let name = self.config_store.extension_name()?;

        Ok(match name.as_str() {
            "pg_analytics" => root.join("deltalake"),
            "pg_search" => root.join("paradedb").join("pg_search"),
            "pg_lakehouse" => root.join("paradedb").join("pg_lakehouse"),
            _ => panic!("no extension_path for unrecognized extension: {name:?}"),
        })
    }

    fn extension_uuid_path(&self) -> Result<PathBuf> {
        Ok(self
            .extension_path()?
            .join(format!("{}_uuid", self.config_store.extension_name()?)))
    }

    fn extension_uuid(&self) -> Result<String> {
        let uuid_file = self.extension_uuid_path()?;
        let uuid_string = fs::read_to_string(&uuid_file)
            .map_err(|err| anyhow!("{err}"))
            .and_then(|s| Uuid::parse_str(&s).map_err(|err| anyhow!("{err}")));
        match uuid_string {
            Ok(uuid) => Ok(uuid.to_string()),
            _ => {
                let new_uuid = Uuid::new_v4().to_string();
                if let Some(parent) = uuid_file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&uuid_file, &new_uuid)?;
                Ok(new_uuid)
            }
        }
    }

    fn extension_size(&self) -> Result<u64> {
        Ok(WalkDir::new(self.extension_path()?)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len()))
    }
}
