use super::{DirectoryStore, TelemetryConfigStore, TelemetryError};
use std::{fs, path::PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct PostgresDirectoryStore {
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl DirectoryStore for PostgresDirectoryStore {
    fn root_path(&self) -> Result<PathBuf, TelemetryError> {
        self.config_store.root_data_directory()
    }

    fn extension_path(&self) -> Result<PathBuf, TelemetryError> {
        Ok(self.root_path()?.join(self.config_store.extension_name()?))
    }

    fn extension_uuid_path(&self) -> Result<PathBuf, TelemetryError> {
        Ok(self
            .extension_path()?
            .join(format!("{}_uuid", self.config_store.extension_name()?)))
    }

    fn extension_uuid(&self) -> Result<String, TelemetryError> {
        let uuid_file = self.extension_uuid_path()?;
        match fs::read_to_string(&uuid_file)
            .map_err(TelemetryError::ReadUuid)
            .and_then(|s| Uuid::parse_str(&s).map_err(TelemetryError::ParseUuid))
        {
            Ok(uuid) => Ok(uuid.to_string()),
            _ => {
                let new_uuid = Uuid::new_v4().to_string();
                if let Some(parent) = uuid_file.parent() {
                    fs::create_dir_all(parent).map_err(TelemetryError::WriteUuid)?;
                }
                fs::write(&uuid_file, &new_uuid).map_err(TelemetryError::WriteUuid)?;
                Ok(new_uuid)
            }
        }
    }

    fn extension_size(&self) -> Result<u64, TelemetryError> {
        Ok(WalkDir::new(self.extension_path()?)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len()))
    }
}
