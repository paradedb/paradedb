// Copyright (c) 2023-2025 ParadeDB, Inc.
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

//! Error types for aggregation operations

use std::error::Error;
use std::fmt;

/// Errors that can occur during aggregation query building and execution
#[derive(Debug)]
pub enum AggregationError {
    /// Invalid pagination parameters
    InvalidPagination { limit: u32, offset: u32 },

    /// No aggregates specified in the query
    NoAggregates,

    /// Error from Tantivy
    Tantivy(tantivy::TantivyError),

    /// Error building Tantivy aggregation
    BuildError(String),

    /// Error converting types
    ConversionError(String),

    /// Generic error with context
    Other(Box<dyn Error>),
}

impl fmt::Display for AggregationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPagination { limit, offset } => {
                write!(
                    f,
                    "Invalid pagination: offset {offset} exceeds limit {limit}"
                )
            }
            Self::NoAggregates => write!(f, "No aggregates specified"),
            Self::Tantivy(e) => write!(f, "Tantivy error: {e}"),
            Self::BuildError(msg) => write!(f, "Build error: {msg}"),
            Self::ConversionError(msg) => write!(f, "Conversion error: {msg}"),
            Self::Other(e) => write!(f, "Aggregation error: {e}"),
        }
    }
}

impl Error for AggregationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Tantivy(e) => Some(e),
            Self::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<tantivy::TantivyError> for AggregationError {
    fn from(err: tantivy::TantivyError) -> Self {
        Self::Tantivy(err)
    }
}

impl From<serde_json::Error> for AggregationError {
    fn from(err: serde_json::Error) -> Self {
        Self::ConversionError(err.to_string())
    }
}

impl From<Box<dyn Error>> for AggregationError {
    fn from(err: Box<dyn Error>) -> Self {
        Self::Other(err)
    }
}

// Note: Conversion to Box<dyn Error> is automatic via the From<Box<dyn Error>> impl
