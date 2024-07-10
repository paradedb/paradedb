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

use super::{Handler, IndexError, SearchFs, ServerError, WriterDirectory, WriterRequest};
use crate::{index::SearchIndex, schema::SearchDocument};
use anyhow::Result;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::sync::{Arc, RwLock};
use tantivy::{schema::Field, IndexWriter};

/// The entity that interfaces with Tantivy indexes.
pub struct Writer {
    /// Map of index directory path to Tantivy writer instance.
    tantivy_writers: HashMap<WriterDirectory, Arc<RwLock<IndexWriter>>>,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            tantivy_writers: HashMap::new(),
        }
    }

    /// Check the writer server cache for an existing IndexWriter. If it does not exist,
    /// then retrieve the SearchIndex and use it to create a new IndexWriter, caching it.
    fn get_writer(&mut self, directory: WriterDirectory) -> Result<Arc<RwLock<IndexWriter>>> {
        let writer = match self.tantivy_writers.entry(directory.clone()) {
            Vacant(entry) => entry.insert(Arc::new(RwLock::new(
                SearchIndex::writer(&directory).map_err(|err| {
                    IndexError::GetWriterFailed(directory.clone(), err.to_string())
                })?,
            ))),
            Occupied(entry) => entry.into_mut(),
        };

        Ok(writer.clone())
    }

    fn insert(&mut self, directory: WriterDirectory, document: SearchDocument) -> Result<()> {
        let writer_lock = self.get_writer(directory)?;
        // Add the Tantivy document to the index.
        writer_lock.write().unwrap().add_document(document.into())?;

        Ok(())
    }

    fn delete(
        &mut self,
        directory: WriterDirectory,
        ctid_field: &Field,
        ctid_values: &[u64],
    ) -> Result<()> {
        let writer_lock = self.get_writer(directory)?;
        let writer = writer_lock.read().unwrap();

        for ctid in ctid_values {
            let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
            writer.delete_term(ctid_term);
        }
        Ok(())
    }

    fn commit(&mut self, directory: WriterDirectory) -> Result<()> {
        if directory.exists()? {
            let writer_lock = self.get_writer(directory)?;
            let mut writer = writer_lock.write().unwrap();
            writer.prepare_commit()?;
            writer.commit()?;
        } else {
            // If the directory doesn't exist, then the index doesn't exist anymore.
            // Rare, but possible if a previous delete failed. Drop it to free the space.
            self.drop_index(directory.clone())?;
        }
        // self.tantivy_writers.remove(&directory);
        Ok(())
    }

    fn abort(&mut self, directory: WriterDirectory) -> Result<()> {
        // If the transaction was aborted, we should drop the writer.
        // Otherwise, partialy written data could stick around for the next transaction.
        self.tantivy_writers.remove(&directory);
        Ok(())
    }

    fn vacuum(&mut self, directory: WriterDirectory) -> Result<()> {
        let writer_lock = self.get_writer(directory)?;
        writer_lock
            .write()
            .unwrap()
            .garbage_collect_files()
            .wait()?;
        Ok(())
    }

    fn drop_index(&mut self, directory: WriterDirectory) -> Result<()> {
        if let Ok(writer) = self.get_writer(directory.clone()) {
            let writer = writer.read().unwrap();
            writer.delete_all_documents()?;
            self.commit(directory.clone())?;

            // Remove the writer from the cache so that it is dropped.
            // We want to do this first so that the lockfile is released before deleting.
            // We'll manually call drop to make sure the lockfile is cleaned up.
            if let Some(writer) = self.tantivy_writers.remove(&directory) {
                std::mem::drop(writer);
            };
        }

        directory.remove()?;

        Ok(())
    }
}

impl Handler<WriterRequest> for Writer {
    fn handle(&mut self, request: WriterRequest) -> Result<(), ServerError> {
        match request {
            WriterRequest::Insert {
                directory,
                document,
            } => self.insert(directory, document).map_err(ServerError::from),
            WriterRequest::Delete {
                directory,
                field,
                ctids,
            } => self
                .delete(directory, &field, &ctids)
                .map_err(ServerError::from),
            WriterRequest::DropIndex { directory } => {
                self.drop_index(directory).map_err(ServerError::from)
            }
            WriterRequest::Commit { directory } => {
                self.commit(directory).map_err(ServerError::from)
            }
            WriterRequest::Abort { directory } => self.abort(directory).map_err(ServerError::from),
            WriterRequest::Vacuum { directory } => {
                self.vacuum(directory).map_err(ServerError::from)
            }
        }
    }
}
