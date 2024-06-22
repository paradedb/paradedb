use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub enum IcebergOption {
    AllowMovedPaths,
    Files,
}

impl IcebergOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AllowMovedPaths => "allow_moved_paths",
            Self::Files => "files",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::AllowMovedPaths => false,
            Self::Files => true,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::AllowMovedPaths, Self::Files].into_iter()
    }
}

pub fn create_view(
    table_name: &str,
    schema_name: &str,
    table_options: HashMap<String, String>,
) -> Result<String> {
    let files = table_options
        .get(IcebergOption::Files.as_str())
        .ok_or_else(|| anyhow!("files option is required"))?;

    let files_str = format!("'{}'", files);

    let allow_moved_paths = table_options
        .get(IcebergOption::AllowMovedPaths.as_str())
        .map(|option| format!("allow_moved_paths = {option}"));

    let create_iceberg_str = [Some(files_str), allow_moved_paths]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(", ");

    Ok(format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM iceberg_scan({create_iceberg_str})"))
}
