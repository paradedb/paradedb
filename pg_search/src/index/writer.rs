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

use crate::{
    index::SearchIndex,
    postgres::types::TantivyValueError,
    schema::{
        SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
    },
};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::{fs, io, result};
use tantivy::directory::{
    DirectoryClone, DirectoryLock, FileHandle, FileSlice, Lock, WatchCallback, WatchHandle,
    WritePtr,
};
use tantivy::{
    directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError},
    IndexSettings,
};
use tantivy::{directory::MmapDirectory, schema::Field, Directory, Index, IndexWriter};
use thiserror::Error;

use super::directory::{SearchDirectoryError, SearchFs, WriterDirectory};

/// We maintain our own tantivy::directory::Directory implementation for finer-grained
/// control over the locking behavior, which enables us to manage Writer instances
/// across multiple connections.
#[derive(Debug)]
pub struct BlockingDirectory(MmapDirectory);

impl BlockingDirectory {
    pub fn open(directory_path: impl AsRef<Path>) -> Result<Self> {
        if !directory_path.as_ref().exists() {
            fs::create_dir_all(&directory_path).expect("must be able to create index directory")
        }
        Ok(Self(MmapDirectory::open(directory_path)?))
    }
}

impl DirectoryClone for BlockingDirectory {
    fn box_clone(&self) -> Box<dyn Directory> {
        self.0.box_clone()
    }
}

impl Directory for BlockingDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        self.0.get_file_handle(path)
    }

    fn open_read(&self, path: &Path) -> result::Result<FileSlice, OpenReadError> {
        self.0.open_read(path)
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        self.0.open_write(path)
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        self.0.atomic_write(path, data)
    }

    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        self.0.atomic_read(path)
    }

    fn delete(&self, path: &Path) -> result::Result<(), DeleteError> {
        self.0.delete(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        self.0.exists(path)
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        // This is the only change we actually need to make to the Directory trait impl.
        // We want the acquire_lock behavior to block and wait for a lock to be available,
        // instead of panicking. Internally, Tantivy just polls for its availability.
        let blocking_lock = Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        };
        self.0.acquire_lock(&blocking_lock)
    }

    fn watch(&self, watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        self.0.watch(watch_callback)
    }

    fn sync_directory(&self) -> io::Result<()> {
        self.0.sync_directory()
    }
}

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub underlying_writer: IndexWriter,
}

impl SearchIndexWriter {
    pub fn insert(&mut self, document: SearchDocument) -> Result<(), IndexError> {
        // Add the Tantivy document to the index.
        self.underlying_writer.add_document(document.into())?;

        Ok(())
    }

    pub fn delete(&mut self, ctid_field: &Field, ctid_values: &[u64]) -> Result<(), IndexError> {
        for ctid in ctid_values {
            let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
            self.underlying_writer.delete_term(ctid_term);
        }
        Ok(())
    }

    pub fn commit(&mut self) -> Result<()> {
        self.underlying_writer
            .prepare_commit()
            .context("error preparing commit to tantivy index")?;
        self.underlying_writer
            .commit()
            .context("error committing to tantivy index")?;

        Ok(())
    }

    pub fn abort(&mut self) -> Result<(), IndexError> {
        self.underlying_writer.rollback()?;

        Ok(())
    }

    pub fn vacuum(&mut self) -> Result<(), IndexError> {
        self.underlying_writer.garbage_collect_files().wait()?;
        Ok(())
    }

    pub fn create_index(
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        uuid: String,
        key_field_index: usize,
    ) -> Result<()> {
        let schema = SearchIndexSchema::new(fields, key_field_index)?;

        let tantivy_dir_path = directory.tantivy_dir_path(true)?;
        let tantivy_dir = BlockingDirectory::open(tantivy_dir_path)?;
        let mut underlying_index =
            Index::create(tantivy_dir, schema.schema.clone(), IndexSettings::default())?;

        SearchIndex::setup_tokenizers(&mut underlying_index, &schema);

        let new_self = SearchIndex {
            underlying_index,
            directory: directory.clone(),
            schema,
            uuid,
            is_pending_drop: false,
            is_pending_create: true,
        };

        // Serialize SearchIndex to disk so it can be initialized by other connections.
        new_self.directory.save_index(&new_self)?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum IndexError {
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
