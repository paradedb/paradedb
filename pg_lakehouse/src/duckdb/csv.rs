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

use super::utils;

pub enum CsvOption {
    AllVarchar,
    AllowQuotedNulls,
    AutoDetect,
    AutoTypeCandidates,
    Columns,
    Compression,
    Dateformat,
    DecimalSeparator,
    Delim,
    Escape,
    Filename,
    Files,
    ForceNotNull,
    Header,
    HivePartitioning,
    HiveTypes,
    HiveTypesAutocast,
    IgnoreErrors,
    MaxLineSize,
    Names,
    NewLine,
    NormalizeNames,
    NullPadding,
    Nullstr,
    Parallel,
    Quote,
    SampleSize,
    Sep,
    Skip,
    Timestampformat,
    Types,
    UnionByName,
}

impl CsvOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AllVarchar => "all_varchar",
            Self::AllowQuotedNulls => "allow_quoted_nulls",
            Self::AutoDetect => "auto_detect",
            Self::AutoTypeCandidates => "auto_type_candidates",
            Self::Columns => "columns",
            Self::Compression => "compression",
            Self::Dateformat => "dateformat",
            Self::DecimalSeparator => "decimal_separator",
            Self::Delim => "delim",
            Self::Escape => "escape",
            Self::Filename => "filename",
            Self::Files => "files",
            Self::ForceNotNull => "force_not_null",
            Self::Header => "header",
            Self::HivePartitioning => "hive_partitioning",
            Self::HiveTypes => "hive_types",
            Self::HiveTypesAutocast => "hive_types_autocast",
            Self::IgnoreErrors => "ignore_errors",
            Self::MaxLineSize => "max_line_size",
            Self::Names => "names",
            Self::NewLine => "new_line",
            Self::NormalizeNames => "normalize_names",
            Self::NullPadding => "null_padding",
            Self::Nullstr => "nullstr",
            Self::Parallel => "parallel",
            Self::Quote => "quote",
            Self::SampleSize => "sample_size",
            Self::Sep => "sep",
            Self::Skip => "skip",
            Self::Timestampformat => "timestampformat",
            Self::Types => "types",
            Self::UnionByName => "union_by_name",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::AllVarchar => false,
            Self::AllowQuotedNulls => false,
            Self::AutoDetect => false,
            Self::AutoTypeCandidates => false,
            Self::Columns => false,
            Self::Compression => false,
            Self::Dateformat => false,
            Self::DecimalSeparator => false,
            Self::Delim => false,
            Self::Escape => false,
            Self::Filename => false,
            Self::Files => true,
            Self::ForceNotNull => false,
            Self::Header => false,
            Self::HivePartitioning => false,
            Self::HiveTypes => false,
            Self::HiveTypesAutocast => false,
            Self::IgnoreErrors => false,
            Self::MaxLineSize => false,
            Self::Names => false,
            Self::NewLine => false,
            Self::NormalizeNames => false,
            Self::NullPadding => false,
            Self::Nullstr => false,
            Self::Parallel => false,
            Self::Quote => false,
            Self::SampleSize => false,
            Self::Sep => false,
            Self::Skip => false,
            Self::Timestampformat => false,
            Self::Types => false,
            Self::UnionByName => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::AllVarchar,
            Self::AllowQuotedNulls,
            Self::AutoDetect,
            Self::AutoTypeCandidates,
            Self::Columns,
            Self::Compression,
            Self::Dateformat,
            Self::DecimalSeparator,
            Self::Delim,
            Self::Escape,
            Self::Filename,
            Self::Files,
            Self::ForceNotNull,
            Self::Header,
            Self::HivePartitioning,
            Self::HiveTypes,
            Self::HiveTypesAutocast,
            Self::IgnoreErrors,
            Self::MaxLineSize,
            Self::Names,
            Self::NewLine,
            Self::NormalizeNames,
            Self::NullPadding,
            Self::Nullstr,
            Self::Parallel,
            Self::Quote,
            Self::SampleSize,
            Self::Sep,
            Self::Skip,
            Self::Timestampformat,
            Self::Types,
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
            .get(CsvOption::Files.as_str())
            .ok_or_else(|| anyhow!("files option is required"))?,
    ));

    let all_varchar = table_options
        .get(CsvOption::AllVarchar.as_str())
        .map(|option| format!("all_varchar = {option}"));

    let allow_quoted_nulls = table_options
        .get(CsvOption::AllowQuotedNulls.as_str())
        .map(|option| format!("allow_quoted_nulls = {option}"));

    let auto_detect = table_options
        .get(CsvOption::AutoDetect.as_str())
        .map(|option| format!("auto_detect = {option}"));

    let auto_type_candidates = table_options
        .get(CsvOption::AutoTypeCandidates.as_str())
        .map(|option| format!("auto_type_candidates = {}", utils::format_csv(option)));

    let columns = table_options
        .get(CsvOption::Columns.as_str())
        .map(|option| format!("columns = {option}"));

    let compression = table_options
        .get(CsvOption::Compression.as_str())
        .map(|option| format!("compression = '{option}'"));

    let dateformat = table_options
        .get(CsvOption::Dateformat.as_str())
        .map(|option| format!("dateformat = '{option}'"));

    let decimal_separator = table_options
        .get(CsvOption::DecimalSeparator.as_str())
        .map(|option| format!("decimal_separator = '{option}'"));

    let delim = table_options
        .get(CsvOption::Delim.as_str())
        .map(|option| format!("delim = '{option}'"));

    let escape = table_options
        .get(CsvOption::Escape.as_str())
        .map(|option| format!("escape = '{option}'"));

    let filename = table_options
        .get(CsvOption::Filename.as_str())
        .map(|option| format!("filename = {option}"));

    let force_not_null = table_options
        .get(CsvOption::ForceNotNull.as_str())
        .map(|option| format!("force_not_null = {}", utils::format_csv(option)));

    let header = table_options
        .get(CsvOption::Header.as_str())
        .map(|option| format!("header = {option}"));

    let hive_partitioning = table_options
        .get(CsvOption::HivePartitioning.as_str())
        .map(|option| format!("hive_partitioning = {option}"));

    let hive_types = table_options
        .get(CsvOption::HiveTypes.as_str())
        .map(|option| format!("hive_types = {option}"));

    let hive_types_autocast = table_options
        .get(CsvOption::HiveTypesAutocast.as_str())
        .map(|option| format!("hive_types_autocast = {option}"));

    let ignore_errors = table_options
        .get(CsvOption::IgnoreErrors.as_str())
        .map(|option| format!("ignore_errors = {option}"));

    let max_line_size = table_options
        .get(CsvOption::MaxLineSize.as_str())
        .map(|option| format!("max_line_size = {option}"));

    let names = table_options
        .get(CsvOption::Names.as_str())
        .map(|option| format!("names = {}", utils::format_csv(option)));

    let new_line = table_options
        .get(CsvOption::NewLine.as_str())
        .map(|option| format!("new_line = '{option}'"));

    let normalize_names = table_options
        .get(CsvOption::NormalizeNames.as_str())
        .map(|option| format!("normalize_names = {option}"));

    let null_padding = table_options
        .get(CsvOption::NullPadding.as_str())
        .map(|option| format!("null_padding = {option}"));

    let nullstr = table_options
        .get(CsvOption::Nullstr.as_str())
        .map(|option| format!("nullstr = {}", utils::format_csv(option)));

    let parallel = table_options
        .get(CsvOption::Parallel.as_str())
        .map(|option| format!("parallel = {option}"));

    let quote = table_options
        .get(CsvOption::Quote.as_str())
        .map(|option| format!("quote = '{option}'"));

    let sample_size = table_options
        .get(CsvOption::SampleSize.as_str())
        .map(|option| format!("sample_size = {option}"));

    let sep = table_options
        .get(CsvOption::Sep.as_str())
        .map(|option| format!("sep = '{option}'"));

    let skip = table_options
        .get(CsvOption::Skip.as_str())
        .map(|option| format!("skip = {option}"));

    let timestampformat = table_options
        .get(CsvOption::Timestampformat.as_str())
        .map(|option| format!("timestampformat = '{option}'"));

    let types = table_options
        .get(CsvOption::Types.as_str())
        .map(|option| format!("types = {}", utils::format_csv(option)));

    let union_by_name = table_options
        .get(CsvOption::UnionByName.as_str())
        .map(|option| format!("union_by_name = {option}"));

    let create_csv_str = vec![
        files,
        all_varchar,
        allow_quoted_nulls,
        auto_detect,
        auto_type_candidates,
        columns,
        compression,
        dateformat,
        decimal_separator,
        delim,
        escape,
        filename,
        force_not_null,
        header,
        hive_partitioning,
        hive_types,
        hive_types_autocast,
        ignore_errors,
        max_line_size,
        names,
        new_line,
        normalize_names,
        null_padding,
        nullstr,
        parallel,
        quote,
        sample_size,
        sep,
        skip,
        timestampformat,
        types,
        union_by_name,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(", ");

    Ok(format!("CREATE VIEW IF NOT EXISTS {schema_name}.{table_name} AS SELECT * FROM read_csv({create_csv_str})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_create_csv_view_single_file() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([(
            CsvOption::Files.as_str().to_string(),
            "/data/file.csv".to_string(),
        )]);
        let expected =
            "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_csv('/data/file.csv')";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid csv file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file.csv")),
        }
    }

    #[test]
    fn test_create_csv_view_multiple_files() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([(
            CsvOption::Files.as_str().to_string(),
            "/data/file1.csv, /data/file2.csv".to_string(),
        )]);

        let expected = "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_csv(['/data/file1.csv', '/data/file2.csv'])";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(&actual) {
            Ok(_) => panic!("invalid csv file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file1.csv")),
        }
    }

    #[test]
    fn test_create_csv_view_with_options() {
        let table_name = "test";
        let schema_name = "main";
        let table_options = HashMap::from([
            (
                CsvOption::Files.as_str().to_string(),
                "/data/file.csv".to_string(),
            ),
            (
                CsvOption::AllVarchar.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::AllowQuotedNulls.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::AutoDetect.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::AutoTypeCandidates.as_str().to_string(),
                "BIGINT, DATE".to_string(),
            ),
            (
                CsvOption::Columns.as_str().to_string(),
                "{'col1': 'INTEGER', 'col2': 'VARCHAR'}".to_string(),
            ),
            (
                CsvOption::Compression.as_str().to_string(),
                "gzip".to_string(),
            ),
            (
                CsvOption::Dateformat.as_str().to_string(),
                "%d/%m/%Y".to_string(),
            ),
            (
                CsvOption::DecimalSeparator.as_str().to_string(),
                ".".to_string(),
            ),
            (CsvOption::Delim.as_str().to_string(), ",".to_string()),
            (CsvOption::Escape.as_str().to_string(), "\"".to_string()),
            (CsvOption::Filename.as_str().to_string(), "true".to_string()),
            (
                CsvOption::ForceNotNull.as_str().to_string(),
                "col1, col2".to_string(),
            ),
            (CsvOption::Header.as_str().to_string(), "true".to_string()),
            (
                CsvOption::HivePartitioning.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::HiveTypes.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::HiveTypesAutocast.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::IgnoreErrors.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::MaxLineSize.as_str().to_string(),
                "1000".to_string(),
            ),
            (
                CsvOption::Names.as_str().to_string(),
                "col1, col2".to_string(),
            ),
            (CsvOption::NewLine.as_str().to_string(), "\n".to_string()),
            (
                CsvOption::NormalizeNames.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::NullPadding.as_str().to_string(),
                "true".to_string(),
            ),
            (
                CsvOption::Nullstr.as_str().to_string(),
                "none, null".to_string(),
            ),
            (CsvOption::Parallel.as_str().to_string(), "true".to_string()),
            (CsvOption::Quote.as_str().to_string(), "\"".to_string()),
            (
                CsvOption::SampleSize.as_str().to_string(),
                "100".to_string(),
            ),
            (CsvOption::Sep.as_str().to_string(), ",".to_string()),
            (CsvOption::Skip.as_str().to_string(), "0".to_string()),
            (
                CsvOption::Timestampformat.as_str().to_string(),
                "yyyy-MM-dd HH:mm:ss".to_string(),
            ),
            (
                CsvOption::Types.as_str().to_string(),
                "BIGINT, VARCHAR".to_string(),
            ),
            (
                CsvOption::UnionByName.as_str().to_string(),
                "true".to_string(),
            ),
        ]);

        let expected = "CREATE VIEW IF NOT EXISTS main.test AS SELECT * FROM read_csv('/data/file.csv', all_varchar = true, allow_quoted_nulls = true, auto_detect = true, auto_type_candidates = ['BIGINT', 'DATE'], columns = {'col1': 'INTEGER', 'col2': 'VARCHAR'}, compression = 'gzip', dateformat = '%d/%m/%Y', decimal_separator = '.', delim = ',', escape = '\"', filename = true, force_not_null = ['col1', 'col2'], header = true, hive_partitioning = true, hive_types = true, hive_types_autocast = true, ignore_errors = true, max_line_size = 1000, names = ['col1', 'col2'], new_line = '\n', normalize_names = true, null_padding = true, nullstr = ['none', 'null'], parallel = true, quote = '\"', sample_size = 100, sep = ',', skip = 0, timestampformat = 'yyyy-MM-dd HH:mm:ss', types = ['BIGINT', 'VARCHAR'], union_by_name = true)";
        let actual = create_view(table_name, schema_name, table_options).unwrap();

        assert_eq!(expected, actual);

        let conn = Connection::open_in_memory().unwrap();
        match conn.prepare(expected) {
            Ok(_) => panic!("invalid csv file should throw an error"),
            Err(e) => assert!(e.to_string().contains("file.csv")),
        }
    }
}
