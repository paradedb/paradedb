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
