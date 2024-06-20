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
    let files_string = match files_split.len() {
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

    let binary_as_string = require_option_or(
        ParquetOption::BinaryAsString.as_str(),
        &table_options,
        "false",
    );
    let file_name = require_option_or(ParquetOption::FileName.as_str(), &table_options, "false");
    let file_row_number = require_option_or(
        ParquetOption::FileRowNumber.as_str(),
        &table_options,
        "false",
    );
    let hive_partitioning = require_option_or(
        ParquetOption::HivePartitioning.as_str(),
        &table_options,
        "false",
    );
    let union_by_name =
        require_option_or(ParquetOption::UnionByName.as_str(), &table_options, "false");

    connection::execute(
        format!(
            r#"
                CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} 
                AS SELECT * FROM read_parquet(
                    {files_string},
                    binary_as_string = {binary_as_string},
                    filename = {file_name},
                    file_row_number = {file_row_number},
                    hive_partitioning = {hive_partitioning},
                    union_by_name = {union_by_name}
                )
            "#,
        )
        .as_str(),
        [],
    )?;

    Ok(())
}
