// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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
    let files = format!(
        "'{}'",
        table_options
            .get(DeltaOption::Files.as_str())
            .ok_or_else(|| anyhow!("files option is required"))?
    );

    Ok(format!(
        "CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM delta_scan({files})"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_create_delta_view() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([(
            DeltaOption::Files.as_str().to_string(),
            "/data/delta".to_string(),
        )]);

        let expected =
            "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM delta_scan('/data/delta')";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid delta file should throw an error"),
            Err(e) => assert!(e.to_string().contains("/data/delta")),
        }
    }
}
