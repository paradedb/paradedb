use super::{Handler, IndexError, SearchFs, ServerError, WriterDirectory, WriterRequest};
use crate::{index::SearchIndex, schema::SearchDocument};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use tantivy::{schema::Field, IndexWriter};

/// The entity that interfaces with Tantivy indexes.
pub struct Writer {
    /// Map of index directory path to Tantivy writer instance.
    tantivy_writers: HashMap<WriterDirectory, tantivy::IndexWriter>,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            tantivy_writers: HashMap::new(),
        }
    }

    /// Check the writer server cache for an existing IndexWriter. If it does not exist,
    /// then retrieve the SearchIndex and use it to create a new IndexWriter, caching it.
    fn get_writer(&mut self, directory: WriterDirectory) -> Result<&mut IndexWriter, IndexError> {
        match self.tantivy_writers.entry(directory.clone()) {
            Vacant(entry) => {
                Ok(entry.insert(SearchIndex::writer(&directory).map_err(|err| {
                    IndexError::GetWriterFailed(directory.clone(), err.to_string())
                })?))
            }
            Occupied(entry) => Ok(entry.into_mut()),
        }
    }

    fn insert(
        &mut self,
        directory: WriterDirectory,
        document: SearchDocument,
    ) -> Result<(), IndexError> {
        let writer = self.get_writer(directory)?;

        // Add the Tantivy document to the index.
        writer.add_document(document.into())?;

        Ok(())
    }

    fn delete(
        &mut self,
        directory: WriterDirectory,
        ctid_field: &Field,
        ctid_values: &[u64],
    ) -> Result<(), IndexError> {
        let writer = self.get_writer(directory)?;
        for ctid in ctid_values {
            let ctid_term = tantivy::Term::from_field_u64(*ctid_field, *ctid);
            writer.delete_term(ctid_term);
        }
        Ok(())
    }

    fn commit(&mut self, directory: WriterDirectory) -> Result<(), IndexError> {
        if directory.exists()? {
            let writer = self
                .get_writer(directory.clone())
                .expect("cannot lookup writer");
            writer.prepare_commit()?;
            writer.commit()?;
        } else {
            // If the directory doesn't exist, then the index doesn't exist anymore.
            // Rare, but possible if a previous delete failed. Drop it to free the space.
            self.drop_index(directory.clone())?;
        }
        self.tantivy_writers.remove(&directory);
        Ok(())
    }

    fn abort(&mut self, directory: WriterDirectory) -> Result<(), IndexError> {
        // If the transaction was aborted, we should drop the writer.
        // Otherwise, partialy written data could stick around for the next transaction.
        self.tantivy_writers.remove(&directory);
        Ok(())
    }

    fn vacuum(&mut self, directory: WriterDirectory) -> Result<(), IndexError> {
        let writer = self.get_writer(directory)?;
        writer.garbage_collect_files().wait()?;
        Ok(())
    }

    fn drop_index(&mut self, directory: WriterDirectory) -> Result<(), IndexError> {
        if let Ok(writer) = self.get_writer(directory.clone()) {
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
