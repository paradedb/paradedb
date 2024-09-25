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

mod directory;
mod index;

use crate::schema::{SearchDocument, SearchFieldConfig, SearchFieldType};
use crate::{postgres::types::TantivyValueError, schema::SearchFieldName};
pub use directory::*;
pub use index::{UnlockedDirectory, Writer};
use serde::{Deserialize, Serialize};
use tantivy::schema::Field;
use thiserror::Error;

// A layer of the client-server request structure that handles
// details about the action to be performed by the index writer.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum WriterRequest {
    Insert {
        directory: WriterDirectory,
        document: SearchDocument,
    },
    Delete {
        directory: WriterDirectory,
        field: Field,
        ctids: Vec<u64>,
    },
    CreateIndex {
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        uuid: String,
        key_field_index: usize,
    },
    DropIndex {
        directory: WriterDirectory,
    },
    Abort {
        directory: WriterDirectory,
    },
    Commit {
        directory: WriterDirectory,
    },
    Vacuum {
        directory: WriterDirectory,
    },
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("couldn't get writer for {0:?}: {1}")]
    GetWriterFailed(WriterDirectory, String),

    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    TantivyValueError(#[from] TantivyValueError),

    #[error("couldn't remove index files on drop_index: {0}")]
    DeleteDirectory(#[from] SearchDirectoryError),

    #[error("key_field column '{0}' cannot be NULL")]
    KeyIdNull(String),
}
