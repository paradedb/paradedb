use anyhow::Result;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;

use super::connection;

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

pub fn create_delta_view(
    table_name: &str,
    schema_name: &str,
    table_options: HashMap<String, String>,
) -> Result<()> {
    let files = require_option(DeltaOption::Files.as_str(), &table_options)?;

    connection::execute(
        format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM delta_scan('{files}')",
        )
        .as_str(),
        [],
    )?;

    Ok(())
}
