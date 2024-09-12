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

pub use crate::writer::SearchFs;
use crate::writer::{
    SearchDirectoryError, TantivyDirPath, WriterDirectory, WriterTransferPipeFilePath,
};
use serde::{de::DeserializeOwned, Serialize};

pub struct MockWriterDirectory {
    pub temp_dir: tempfile::TempDir,
    pub writer_dir: WriterDirectory,
}

impl MockWriterDirectory {
    pub fn new(index_oid: u32) -> Self {
        // We must store the TempDir instance on the struct, because it gets deleted when the
        // instance is dropped.
        let temp_dir = tempfile::Builder::new()
            .prefix(&index_oid.to_string())
            .tempdir()
            .expect("error creating tempdir for MockWriterDirectory");
        let temp_path = temp_dir.path().to_path_buf();
        Self {
            temp_dir,
            writer_dir: WriterDirectory {
                database_oid: 0, // mock value for test
                index_oid,
                relfile_oid: 0, // mock value for test
                postgres_data_dir_path: temp_path,
            },
        }
    }
}

impl SearchFs for MockWriterDirectory {
    fn load_index<T: DeserializeOwned>(&self) -> Result<T, SearchDirectoryError> {
        self.writer_dir.load_index()
    }
    fn save_index<T: Serialize>(&self, index: &T) -> Result<(), SearchDirectoryError> {
        self.writer_dir.save_index(index)
    }
    fn remove(&self) -> Result<(), SearchDirectoryError> {
        self.writer_dir.remove()
    }
    fn tantivy_dir_path(
        &self,
        ensure_exists: bool,
    ) -> Result<TantivyDirPath, SearchDirectoryError> {
        self.writer_dir.tantivy_dir_path(ensure_exists)
    }
    fn writer_transfer_pipe_path(
        &self,
        ensure_exists: bool,
    ) -> Result<WriterTransferPipeFilePath, SearchDirectoryError> {
        self.writer_dir.writer_transfer_pipe_path(ensure_exists)
    }
}
