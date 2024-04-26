use datafusion::datasource::file_format::{
    avro::AvroFormat, csv::CsvFormat, json::JsonFormat, parquet::ParquetFormat,
};
use datafusion::datasource::listing::ListingOptions;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct FileFormat(pub String);

impl TryFrom<FileFormat> for ListingOptions {
    type Error = FileFormatError;

    fn try_from(format: FileFormat) -> Result<Self, FileFormatError> {
        let FileFormat(format) = format;

        let listing_options = match format.to_lowercase().as_str() {
            "avro" => {
                ListingOptions::new(Arc::new(AvroFormat::default())).with_file_extension(".avro")
            }
            "csv" => {
                ListingOptions::new(Arc::new(CsvFormat::default())).with_file_extension(".csv")
            }
            "json" => {
                ListingOptions::new(Arc::new(JsonFormat::default())).with_file_extension(".json")
            }
            "parquet" => ListingOptions::new(Arc::new(ParquetFormat::default()))
                .with_file_extension(".parquet"),
            unsupported => return Err(FileFormatError::InvalidFileFormat(unsupported.to_string())),
        };

        Ok(listing_options)
    }
}

#[derive(Error, Debug)]
pub enum FileFormatError {
    #[error("Invalid format {0}. Options are avro, csv, json, and parquet.")]
    InvalidFileFormat(String),
}
