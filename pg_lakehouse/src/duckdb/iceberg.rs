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
    let files = Some(format!(
        "'{}'",
        table_options
            .get(IcebergOption::Files.as_str())
            .ok_or_else(|| anyhow!("files option is required"))?
    ));

    let allow_moved_paths = table_options
        .get(IcebergOption::AllowMovedPaths.as_str())
        .map(|option| format!("allow_moved_paths = {option}"));

    let create_iceberg_str = [files, allow_moved_paths]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(", ");

    Ok(format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM iceberg_scan({create_iceberg_str})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_create_iceberg_view() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([(
            IcebergOption::Files.as_str().to_string(),
            "/data/iceberg".to_string(),
        )]);

        let expected =
            "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM iceberg_scan('/data/iceberg')";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("INSTALL iceberg; LOAD iceberg;")
            .unwrap();

        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid iceberg file should throw an error"),
            Err(e) => assert!(e.to_string().contains("/data/iceberg")),
        }
    }
}
