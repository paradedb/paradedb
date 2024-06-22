use anyhow::{anyhow, Result};
use std::collections::HashMap;

use super::utils;

pub enum ParquetOption {
    BinaryAsString,
    FileName,
    FileRowNumber,
    Files,
    HivePartitioning,
    HiveTypes,
    HiveTypesAutocast,
    UnionByName,
    // TODO: EncryptionConfig
}

impl ParquetOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::BinaryAsString => "binary_as_string",
            Self::FileName => "file_name",
            Self::FileRowNumber => "file_row_number",
            Self::Files => "files",
            Self::HivePartitioning => "hive_partitioning",
            Self::HiveTypes => "hive_types",
            Self::HiveTypesAutocast => "hive_types_autocast",
            Self::UnionByName => "union_by_name",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::BinaryAsString => false,
            Self::FileName => false,
            Self::FileRowNumber => false,
            Self::Files => true,
            Self::HivePartitioning => false,
            Self::HiveTypes => false,
            Self::HiveTypesAutocast => false,
            Self::UnionByName => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::BinaryAsString,
            Self::FileName,
            Self::FileRowNumber,
            Self::Files,
            Self::HivePartitioning,
            Self::HiveTypes,
            Self::HiveTypesAutocast,
            Self::UnionByName,
        ]
        .into_iter()
    }
}

pub fn create_view(
    table_name: &str,
    schema_name: &str,
    table_options: HashMap<String, String>,
) -> Result<String> {
    let files = Some(utils::format_csv(
        table_options
            .get(ParquetOption::Files.as_str())
            .ok_or_else(|| anyhow!("files option is required"))?,
    ));

    let binary_as_string = table_options
        .get(ParquetOption::BinaryAsString.as_str())
        .map(|option| format!("binary_as_string = {option}"));

    let file_name = table_options
        .get(ParquetOption::FileName.as_str())
        .map(|option| format!("filename = {option}"));

    let file_row_number = table_options
        .get(ParquetOption::FileRowNumber.as_str())
        .map(|option| format!("file_row_number = {option}"));

    let hive_partitioning = table_options
        .get(ParquetOption::HivePartitioning.as_str())
        .map(|option| format!("hive_partitioning = {option}"));

    let hive_types = table_options
        .get(ParquetOption::HiveTypes.as_str())
        .map(|option| format!("hive_types = {option}"));

    let hive_types_autocast = table_options
        .get(ParquetOption::HiveTypesAutocast.as_str())
        .map(|option| format!("hive_types_autocast = {option}"));

    let union_by_name = table_options
        .get(ParquetOption::UnionByName.as_str())
        .map(|option| format!("union_by_name = {option}"));

    let create_parquet_str = [
        files,
        binary_as_string,
        file_name,
        file_row_number,
        hive_partitioning,
        hive_types,
        hive_types_autocast,
        union_by_name,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(", ");

    Ok(format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM read_parquet({create_parquet_str})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_create_parquet_view_single_file() {
        let table_name = "test";
        let schema_name = "main";
        let files = "/data/file.parquet";
        let table_options =
            HashMap::from([(ParquetOption::Files.as_str().to_string(), files.to_string())]);
        let expected = "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_parquet('/data/file.parquet')";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid parquet file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file.parquet")),
        }
    }

    #[test]
    fn test_create_parquet_view_multiple_files() {
        let table_name = "test";
        let schema_name = "main";
        let files = "/data/file1.parquet, /data/file2.parquet";
        let table_options =
            HashMap::from([(ParquetOption::Files.as_str().to_string(), files.to_string())]);

        let expected = "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_parquet(['/data/file1.parquet', '/data/file2.parquet'])";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid parquet file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file1.parquet")),
        }
    }

    #[test]
    fn test_create_parquet_view_with_options() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([
            (
                ParquetOption::Files.as_str().to_string(),
                "/data/file.parquet".to_string(),
            ),
            (
                ParquetOption::BinaryAsString.as_str().to_string(),
                "true".to_string(),
            ),
            (
                ParquetOption::FileName.as_str().to_string(),
                "false".to_string(),
            ),
            (
                ParquetOption::FileRowNumber.as_str().to_string(),
                "true".to_string(),
            ),
            (
                ParquetOption::HivePartitioning.as_str().to_string(),
                "true".to_string(),
            ),
            (
                ParquetOption::HiveTypes.as_str().to_string(),
                "{'release': DATE, 'orders': BIGINT}".to_string(),
            ),
            (
                ParquetOption::HiveTypesAutocast.as_str().to_string(),
                "true".to_string(),
            ),
            (
                ParquetOption::UnionByName.as_str().to_string(),
                "true".to_string(),
            ),
        ]);

        let expected = "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_parquet('/data/file.parquet', binary_as_string = true, filename = false, file_row_number = true, hive_partitioning = true, hive_types = {'release': DATE, 'orders': BIGINT}, hive_types_autocast = true, union_by_name = true)";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&expected) {
            Ok(_) => panic!("invalid parquet file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file.parquet")),
        }
    }
}
