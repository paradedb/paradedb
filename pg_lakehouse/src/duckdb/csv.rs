use anyhow::Result;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;

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
    let files = require_option(CsvOption::Files.as_str(), &table_options)?;
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
        .map(|option| format!("auto_type_candidates = {option}"));

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
        .map(|option| format!("force_not_null = {option}"));

    let header = table_options
        .get(CsvOption::Header.as_str())
        .map(|option| format!("header = {option}"));

    let hive_partitioning = table_options
        .get(CsvOption::HivePartitioning.as_str())
        .map(|option| format!("hive_partitioning = {option}"));

    let ignore_errors = table_options
        .get(CsvOption::IgnoreErrors.as_str())
        .map(|option| format!("ignore_errors = {option}"));

    let max_line_size = table_options
        .get(CsvOption::MaxLineSize.as_str())
        .map(|option| format!("max_line_size = {option}"));

    let names = table_options
        .get(CsvOption::Names.as_str())
        .map(|option| format!("names = {option}"));

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
        .map(|option| format!("nullstr = '{option}'"));

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
        .map(|option| format!("types = '{option}'"));

    let union_by_name = table_options
        .get(CsvOption::UnionByName.as_str())
        .map(|option| format!("union_by_name = {option}"));

    let create_csv_str = vec![
        Some(files_str),
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
