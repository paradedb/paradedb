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
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{collections::HashSet, path::Path};
use std::{fs, io, result};
use tantivy::directory::{
    AntiCallToken, DirectoryClone, DirectoryLock, FileHandle, FileSlice, Lock, WatchCallback,
    WatchHandle, WritePtr,
};
use tantivy::{
    directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError},
    IndexSettings,
};
use tantivy::{
    directory::MmapDirectory, schema::Field, Directory, Index, SingleSegmentIndexWriter,
};
use thiserror::Error;

use super::directory::{SearchDirectoryError, SearchFs, WriterDirectory};
use crate::postgres::storage::atomic::AtomicDirectory;
use crate::postgres::storage::segment_reader::SegmentReader;
use crate::postgres::storage::segment_writer::SegmentWriter;

/// We maintain our own tantivy::directory::Directory implementation for finer-grained
/// control over the locking behavior, which enables us to manage Writer instances
/// across multiple connections.
#[derive(Clone, Debug)]
pub struct BlockingDirectory {
    relation_oid: u32,
}

impl BlockingDirectory {
    pub fn new(relation_oid: u32) -> Self {
        Self { relation_oid }
    }
}

impl Directory for BlockingDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        Ok(Arc::new(unsafe {
            SegmentReader::new(self.relation_oid, path).map_err(|e| {
                OpenReadError::wrap_io_error(
                    io::Error::new(io::ErrorKind::Other, format!("{:?}", e)),
                    path.to_path_buf(),
                )
            })?
        }))
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        pgrx::info!("open_write: {:?}", path);
        Ok(io::BufWriter::new(Box::new(unsafe {
            SegmentWriter::new(self.relation_oid, path)
        })))
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        pgrx::info!("atomic_write: {:?}", path);
        let directory = unsafe { AtomicDirectory::new(self.relation_oid) };
        match path.to_str().unwrap().ends_with("meta.json") {
            true => unsafe { directory.write_meta(data) },
            false => unsafe { directory.write_managed(data) },
        };

        Ok(())
    }

    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        pgrx::info!("atomic_read: {:?}", path);
        let directory = unsafe { AtomicDirectory::new(self.relation_oid) };
        let data = match path.to_str().unwrap().ends_with("meta.json") {
            true => unsafe { directory.read_meta() },
            false => unsafe { directory.read_managed() },
        };

        if data.is_empty() {
            return Err(OpenReadError::FileDoesNotExist(PathBuf::from(path)));
        }

        Ok(data)
    }

    fn delete(&self, path: &Path) -> result::Result<(), DeleteError> {
        todo!("delete");
        // pgrx::info!("delete: {:?}", path);
        // self.0.delete(path)
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        todo!("exists");
        // pgrx::info!("exists: {:?}", path);
        // self.0.exists(path)
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        pgrx::info!("acquire_lock: {:?}", lock.filepath);
        // The lock itself doesn't seem to actually be used anywhere by Tantivy
        Ok(DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
    }

    fn watch(&self, watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        todo!("watch");
        // pgrx::info!("watch");
        // self.0.watch(watch_callback)
    }

    fn sync_directory(&self) -> io::Result<()> {
        pgrx::info!("sync_directory");
        Ok(())
        // pgrx::info!("sync_directory");
        // self.0.sync_directory()
    }
}

/// A global store of which indexes have been created during a transaction,
/// so that they can be committed or rolled back in case of an abort.
static mut PENDING_INDEX_CREATES: Lazy<HashSet<WriterDirectory>> = Lazy::new(HashSet::new);

/// A global store of which indexes have been dropped during a transaction,
/// so that they can be committed or rolled back in case of an abort.
static mut PENDING_INDEX_DROPS: Lazy<HashSet<WriterDirectory>> = Lazy::new(HashSet::new);

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    // this is an Option<> because on drop we need to take ownership of the underlying
    // IndexWriter instance so we can, in the background, wait for all merging threads to finish
    pub underlying_writer: Option<SingleSegmentIndexWriter>,
}

impl SearchIndexWriter {
    pub fn insert(&mut self, document: SearchDocument) -> Result<(), IndexError> {
        // Add the Tantivy document to the index.
        self.underlying_writer
            .as_mut()
            .unwrap()
            .add_document(document.into())?;

        Ok(())
    }

    pub fn delete(&self, ctid_field: &Field, ctid_values: &[u64]) -> Result<(), IndexError> {
        // for ctid in ctid_values {
        //     let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
        //     self.underlying_writer
        //         .as_ref()
        //         .unwrap()
        //         .delete_term(ctid_term);
        // }
        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        pgrx::info!("committing");
        self.underlying_writer
            .unwrap()
            .finalize()
            .context("error committing to tantivy index")?;
        pgrx::info!("committed");
        Ok(())
    }

    pub fn abort(&mut self) -> Result<(), IndexError> {
        // self.underlying_writer.as_mut().unwrap().rollback()?;
        Ok(())
    }

    pub fn vacuum(&self) -> Result<(), IndexError> {
        // self.underlying_writer
        //     .as_ref()
        //     .unwrap()
        //     .garbage_collect_files()
        //     .wait()?;
        Ok(())
    }

    pub fn create_index(
        directory: WriterDirectory,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Result<()> {
        let schema = SearchIndexSchema::new(fields, key_field_index)?;

        let tantivy_dir_path = directory.tantivy_dir_path(true)?;
        let tantivy_dir = BlockingDirectory::new(directory.index_oid);
        let mut underlying_index =
            Index::create(tantivy_dir, schema.schema.clone(), IndexSettings::default())?;

        SearchIndex::setup_tokenizers(&mut underlying_index, &schema);

        let new_self = SearchIndex {
            underlying_index,
            directory: directory.clone(),
            schema,
        };

        // Serialize SearchIndex to disk so it can be initialized by other connections.
        new_self.directory.save_index(&new_self)?;

        // Mark in our global store that this index is pending create, in case it
        // needs to be rolled back on abort.
        Self::mark_pending_create(&directory);

        Ok(())
    }

    pub fn mark_pending_create(directory: &WriterDirectory) -> bool {
        unsafe { PENDING_INDEX_CREATES.insert(directory.clone()) }
    }

    pub fn mark_pending_drop(directory: &WriterDirectory) -> bool {
        unsafe { PENDING_INDEX_DROPS.insert(directory.clone()) }
    }

    pub fn clear_pending_creates() {
        unsafe { PENDING_INDEX_CREATES.clear() }
    }

    pub fn clear_pending_drops() {
        unsafe { PENDING_INDEX_DROPS.clear() }
    }

    pub fn pending_creates() -> impl Iterator<Item = &'static WriterDirectory> {
        unsafe { PENDING_INDEX_CREATES.iter() }
    }

    pub fn pending_drops() -> impl Iterator<Item = &'static WriterDirectory> {
        unsafe { PENDING_INDEX_DROPS.iter() }
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
