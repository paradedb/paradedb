use super::{DirectoryStore, TelemetryError};
use std::{fs, path::PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct PostgresDirectoryStore {
    extension_name: String,
    root_path: PathBuf,
}

impl PostgresDirectoryStore {
    pub fn new(extension_name: &str) -> Result<Self, TelemetryError> {
        let root_path = std::env::var("PGDATA")
            .map(PathBuf::from)
            .map_err(TelemetryError::NoPgData)?;
        Ok(Self {
            extension_name: extension_name.to_string(),
            root_path,
        })
    }
}

impl DirectoryStore for PostgresDirectoryStore {
    type Error = TelemetryError;

    fn root_path(&self) -> Result<PathBuf, Self::Error> {
        Ok(self.root_path.clone())
    }

    fn extension_path(&self) -> Result<PathBuf, Self::Error> {
        Ok(self.root_path()?.join(&self.extension_name))
    }

    fn extension_uuid_path(&self) -> Result<PathBuf, Self::Error> {
        Ok(self
            .extension_path()?
            .join(format!("{}_uuid", self.extension_name)))
    }

    fn extension_uuid(&self) -> Result<String, Self::Error> {
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

    fn extension_size(&self) -> Result<u64, Self::Error> {
        Ok(WalkDir::new(self.extension_path()?)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .fold(0, |acc, m| acc + m.len()))
    }
}
