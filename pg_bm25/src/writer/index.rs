#![allow(unused_variables, unused_mut, unused_imports)]
use super::{
    entry::{IndexEntry, IndexKey},
    Handler, IndexError, ServerError, WriterRequest,
};
use crate::parade_index::index::ParadeIndex;
use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    fs,
    path::Path,
};
use tantivy::{
    schema::{Field, Value},
    Document, IndexWriter,
};

/// The entity that interfaces with Tantivy indexes.
pub struct Writer {
    /// Map of index directory path to Tantivy writer instance.
    tantivy_writers: HashMap<String, tantivy::IndexWriter>,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            tantivy_writers: HashMap::new(),
        }
    }

    /// Check the writer server cache for an existing IndexWriter. If it does not exist,
    /// then retrieve the ParadeIndex and use it to create a new IndexWriter, caching it.
    fn get_writer(&mut self, index_directory_path: &str) -> Result<&mut IndexWriter, IndexError> {
        match self.tantivy_writers.entry(index_directory_path.to_string()) {
            Vacant(entry) => Ok(
                entry.insert(ParadeIndex::writer(index_directory_path).map_err(|err| {
                    IndexError::GetWriterFailed(index_directory_path.to_string(), err.to_string())
                })?),
            ),
            Occupied(entry) => Ok(entry.into_mut()),
        }
    }

    fn insert(
        &mut self,
        index_directory_path: &str,
        index_entries: Vec<IndexEntry>,
        key_field: IndexKey,
    ) -> Result<(), IndexError> {
        let writer = self.get_writer(index_directory_path)?;

        // Add each of the fields to the Tantivy document.
        let mut doc: Document = Document::new();
        // for entry in index_entries {
        //     // Delete any exiting documents with the same key.
        //     if entry.key == key_field {
        //         writer.delete_term(entry.clone().into());
        //     }

        //     let tantivy_value: Value = entry.value.try_into()?;
        //     doc.add_field_value(entry.key, tantivy_value);
        // }

        // // Add the Tantivy document to the index.
        writer.add_document(doc)?;

        Ok(())
    }

    fn delete(
        &mut self,
        index_directory_path: &str,
        ctid_field: &Field,
        ctid_values: &[u64],
    ) -> Result<(), IndexError> {
        let writer = self.get_writer(index_directory_path)?;
        for ctid in ctid_values {
            let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
            writer.delete_term(ctid_term);
        }
        Ok(())
    }

    fn commit(&mut self) -> Result<(), IndexError> {
        for writer in self.tantivy_writers.values_mut() {
            writer.prepare_commit()?;
            writer.commit()?;
        }
        Ok(())
    }

    fn abort(&mut self) -> Result<(), IndexError> {
        // If the transaction was aborted, we should clear all the writers from the cache.
        // Otherwise, partialy written data could stick around for the next transaction.
        self.tantivy_writers.drain();
        Ok(())
    }

    fn vacuum(&mut self, index_directory_path: &str) -> Result<(), IndexError> {
        let writer = self.get_writer(index_directory_path)?;
        writer.garbage_collect_files().wait()?;
        Ok(())
    }

    fn drop_index<T: AsRef<str>>(
        &mut self,
        index_directory_path: &str,
        paths_to_delete: &[T],
    ) -> Result<(), IndexError> {
        if let Ok(writer) = self.get_writer(index_directory_path) {
            if std::path::Path::new(&index_directory_path).exists() {
                writer.delete_all_documents()?;
                self.commit()?;
            }

            // Remove the writer from the cache so that it is dropped.
            // We want to do this first so that the lockfile is released before deleting.
            // We'll manually call drop to make sure the lockfile is cleaned up.
            if let Some(writer) = self.tantivy_writers.remove(index_directory_path) {
                std::mem::drop(writer);
            };

            // Filter out non-existent paths and sort: files first, then directories.
            let mut paths_to_delete: Vec<&str> =
                paths_to_delete.iter().map(|p| p.as_ref()).collect();
            paths_to_delete.retain(|path| Path::new(path).exists());
            paths_to_delete.sort_by_key(|path| !Path::new(path).is_file());

            // Iterate through the sorted list and delete each path.
            for path in paths_to_delete {
                // Even though we've filtered out the files that supposedly don't exist above,
                // we can still see errors around files existing/not existing unexpectedly.
                // we'll just check again here to be safe.
                let path_ref = Path::new(&path);
                if path_ref.try_exists()? {
                    if path_ref.is_file() {
                        fs::remove_file(path_ref)?;
                    } else {
                        fs::remove_dir_all(path_ref)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Handler<WriterRequest> for Writer {
    fn handle(&mut self, request: WriterRequest) -> Result<(), ServerError> {
        match request {
            WriterRequest::Insert {
                index_directory_path,
                index_entries,
                key_field,
            } => self
                .insert(&index_directory_path, index_entries, key_field)
                .map_err(ServerError::from),
            WriterRequest::Delete {
                index_directory_path,
                field,
                ctids,
            } => self
                .delete(&index_directory_path, &field, &ctids)
                .map_err(ServerError::from),
            WriterRequest::DropIndex {
                index_directory_path,
                paths_to_delete,
            } => self
                .drop_index(&index_directory_path, &paths_to_delete)
                .map_err(ServerError::from),
            WriterRequest::Commit => self.commit().map_err(ServerError::from),
            WriterRequest::Abort => self.abort().map_err(ServerError::from),
            WriterRequest::Vacuum {
                index_directory_path,
            } => self
                .vacuum(&index_directory_path)
                .map_err(ServerError::from),
        }
    }
}
