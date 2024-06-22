use anyhow::{anyhow, Result};
use std::collections::HashMap;

pub enum DeltaOption {
    Files,
}

impl DeltaOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Files => "files",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Files => true,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::Files].into_iter()
    }
}

pub fn create_view(
    table_name: &str,
    schema_name: &str,
    table_options: HashMap<String, String>,
) -> Result<String> {
    let files = table_options
        .get(DeltaOption::Files.as_str())
        .ok_or_else(|| anyhow!("files option is required"))?;

    Ok(format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM delta_scan('{files}')"))
}
