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

use datafusion::datasource::file_format::{
    avro::AvroFormat, csv::CsvFormat, json::JsonFormat, parquet::ParquetFormat,
};
use datafusion::datasource::listing::ListingOptions;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct FileExtension(pub String);

#[derive(PartialEq, Clone, Debug)]
pub enum TableFormat {
    None,
    Delta,
}

impl TableFormat {
    pub fn from(format: &str) -> Self {
        match format {
            "" => Self::None,
            "delta" => Self::Delta,
            _ => Self::None,
        }
    }
}

impl TryFrom<FileExtension> for ListingOptions {
    type Error = FormatError;

    fn try_from(format: FileExtension) -> Result<Self, FormatError> {
        let FileExtension(format) = format;

        let listing_options = match format.to_lowercase().as_str() {
            "avro" => ListingOptions::new(Arc::new(AvroFormat)).with_file_extension(".avro"),
            "csv" => {
                ListingOptions::new(Arc::new(CsvFormat::default())).with_file_extension(".csv")
            }
            "json" => {
                ListingOptions::new(Arc::new(JsonFormat::default())).with_file_extension(".json")
            }
            "parquet" => ListingOptions::new(Arc::new(ParquetFormat::default()))
                .with_file_extension(".parquet"),
            unsupported => return Err(FormatError::InvalidFileFormat(unsupported.to_string())),
        };

        Ok(listing_options)
    }
}

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("Invalid format {0}. Options are avro, csv, json, and parquet.")]
    InvalidFileFormat(String),
}
