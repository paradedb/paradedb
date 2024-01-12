use super::{server::ServerError, Handler, WriterRequest};
use crate::{
    json::builder::{JsonBuilder, JsonBuilderValue},
    parade_index::index::ParadeIndex,
};
use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    fs,
    path::Path,
};
use tantivy::{schema::Field, Document, IndexWriter, Term};

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
    fn get_writer(&mut self, index_directory_path: &str) -> Result<&mut IndexWriter, ServerError> {
        match self.tantivy_writers.entry(index_directory_path.to_string()) {
            Vacant(entry) => Ok(
                entry.insert(ParadeIndex::writer(index_directory_path).map_err(|err| {
                    ServerError::GetWriterFailed(index_directory_path.to_string(), err.to_string())
                })?),
            ),
            Occupied(entry) => Ok(entry.into_mut()),
        }
    }

    fn insert(
        &mut self,
        index_directory_path: &str,
        json_builder: JsonBuilder,
    ) -> Result<(), ServerError> {
        let key_field = json_builder.key;
        let key_value: i64 = match json_builder.values.get(&key_field) {
            Some(JsonBuilderValue::i16(value)) => *value as i64,
            Some(JsonBuilderValue::i32(value)) => *value as i64,
            Some(JsonBuilderValue::i64(value)) => *value,
            Some(JsonBuilderValue::u32(value)) => *value as i64,
            Some(JsonBuilderValue::u64(value)) => *value as i64,
            _ => return Err(ServerError::InvalidKeyField),
        };

        let writer = self.get_writer(index_directory_path)?;

        // Add each of the fields to the Tantivy document.
        let mut doc: Document = Document::new();
        for (field, value) in json_builder.values.iter() {
            value.add_to_tantivy_doc(&mut doc, field);
        }

        // Delete any exiting documents with the same key.
        let key_term = Term::from_field_i64(key_field, key_value);
        writer.delete_term(key_term);

        // Add the Tantivy document to the index.
        writer.add_document(doc)?;

        Ok(())
    }

    fn delete(
        &mut self,
        index_directory_path: &str,
        ctid_field: &Field,
        ctid_values: &[u64],
    ) -> Result<(), ServerError> {
        let writer = self.get_writer(index_directory_path)?;
        for ctid in ctid_values {
            let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
            writer.delete_term(ctid_term);
        }
        Ok(())
    }

    fn commit(&mut self) -> Result<(), ServerError> {
        for (path, writer) in self.tantivy_writers.iter_mut() {
            writer.prepare_commit()?;
            writer.commit()?;
        }
        Ok(())
    }

    fn abort(&mut self) -> Result<(), ServerError> {
        // If the transaction was aborted, we should clear all the writers from the cache.
        // Otherwise, partialy written data could stick around for the next transaction.
        self.tantivy_writers.drain();
        Ok(())
    }

    fn vacuum(&mut self, index_directory_path: &str) -> Result<(), ServerError> {
        let writer = self.get_writer(index_directory_path)?;
        writer.garbage_collect_files().wait()?;
        Ok(())
    }

    fn drop_index<T: AsRef<str>>(
        &mut self,
        index_directory_path: &str,
        paths_to_delete: &[T],
    ) -> Result<(), ServerError> {
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
                json_builder,
            } => self.insert(&index_directory_path, json_builder),
            WriterRequest::Delete {
                index_directory_path,
                field,
                ctids,
            } => self.delete(&index_directory_path, &field, &ctids),
            WriterRequest::DropIndex {
                index_directory_path,
                paths_to_delete,
            } => self.drop_index(&index_directory_path, &paths_to_delete),
            WriterRequest::Commit => self.commit(),
            WriterRequest::Abort => self.abort(),
            WriterRequest::Vacuum {
                index_directory_path,
            } => self.vacuum(&index_directory_path),
        }
    }
}
