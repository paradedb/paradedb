use anyhow::Result;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;

use super::connection;

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

pub fn create_parquet_view(
    table_name: &str,
    schema_name: &str,
    table_options: HashMap<String, String>,
) -> Result<()> {
    let files = require_option(ParquetOption::Files.as_str(), &table_options)?;
    let files_split = files.split(',').collect::<Vec<&str>>();
    let files_str = match files_split.len() {
        1 => format!("'{}'", files),
        _ => format!(
            "[{}]",
            files_split
                .iter()
                .map(|&chunk| format!("'{}'", chunk))
                .collect::<Vec<String>>()
                .join(", ")
        ),
    };

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
        Some(files_str),
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

    connection::execute(
        format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM read_parquet({create_parquet_str})",
        )
        .as_str(),
        [],
    )?;

    Ok(())
}
