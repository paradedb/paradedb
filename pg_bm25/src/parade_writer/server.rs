use crate::json::builder::JsonBuilderValue;
use crate::parade_writer::{ParadeWriterRequest, ParadeWriterResponse};
use crate::{json::builder::JsonBuilder, parade_index::index::ParadeIndex};
use std::collections::{hash_map::Entry::Vacant, HashMap};
use std::fs;
use std::path::Path;
use tantivy::schema::Field;
use tantivy::{Document, IndexWriter, Term};

pub struct ParadeWriterServer {
    should_exit: bool,
    writers: HashMap<String, IndexWriter>,
}

impl ParadeWriterServer {
    pub fn new() -> Self {
        Self {
            should_exit: false,
            writers: HashMap::new(),
        }
    }

    /// If the server receives a shutdown message, then this method should
    /// return true, and the server will be shutdown wherever it has been
    /// initialized.
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// A displach method to choose an action based on a variant of ParadeWriterRequest.
    pub fn handle(&mut self, request: ParadeWriterRequest) -> ParadeWriterResponse {
        match request {
            ParadeWriterRequest::Shutdown => self.shutdown(),
            ParadeWriterRequest::Commit(index_directory_path) => self.commit(index_directory_path),
            ParadeWriterRequest::Insert(index_directory_path, json_builder) => {
                self.insert(index_directory_path, json_builder)
            }
            ParadeWriterRequest::Delete(index_directory_path, ctid_field, ctid_values) => {
                self.delete(index_directory_path, ctid_field, ctid_values)
            }
            ParadeWriterRequest::DropIndex(index_directory_path, paths_to_delete) => {
                self.drop_index(index_directory_path, paths_to_delete)
            }
            ParadeWriterRequest::Vacuum(index_directory_path) => self.vacuum(index_directory_path),
        }
    }

    /// Check the writer server cache for an existing IndexWriter. If it does not exist,
    /// then retrieve the ParadeIndex and use it to create a new IndexWriter, caching it.
    fn writer(&mut self, index_directory_path: &str) -> Result<&mut IndexWriter, String> {
        if let Vacant(entry) = self.writers.entry(index_directory_path.to_string()) {
            if let Err(e) =
                ParadeIndex::writer(index_directory_path).map(|writer| entry.insert(writer))
            {
                return Err(e.to_string());
            }
        }
        self.writers.get_mut(index_directory_path).ok_or(format!(
            "parade writer server could not find associated writer for {index_directory_path:?}",
        ))
    }

    /// Insert one row of fields into the Tantivy index as a new document.
    fn insert(
        &mut self,
        index_directory_path: String,
        json_builder: JsonBuilder,
    ) -> ParadeWriterResponse {
        let key_field = json_builder.key;
        let key_value: i64 = match json_builder.values.get(&key_field) {
            Some(JsonBuilderValue::i16(value)) => *value as i64,
            Some(JsonBuilderValue::i32(value)) => *value as i64,
            Some(JsonBuilderValue::i64(value)) => *value,
            Some(JsonBuilderValue::u32(value)) => *value as i64,
            Some(JsonBuilderValue::u64(value)) => *value as i64,
            _ => {
                return ParadeWriterResponse::Error(format!(
                    "only integer types are supported for the key field, received: {:?}",
                    json_builder.values.get(&key_field)
                ));
            }
        };

        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => {
                // Add each of the fields to the Tantivy document.
                let mut doc: Document = Document::new();
                for (field, value) in json_builder.values.iter() {
                    value.add_to_tantivy_doc(&mut doc, field);
                }

                // Delete any exiting documents with the same key.
                let key_term = Term::from_field_i64(key_field, key_value);
                writer.delete_term(key_term);

                // Add the Tantivy document to the index.
                if let Err(e) = writer.add_document(doc) {
                    let msg = format!("error adding document to tantivy index: {e:?}");
                    return ParadeWriterResponse::Error(msg);
                }

                ParadeWriterResponse::Ok
            }
        }
    }

    fn delete(
        &mut self,
        index_directory_path: String,
        ctid_field: Field,
        ctid_values: Vec<u64>,
    ) -> ParadeWriterResponse {
        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => {
                for ctid in ctid_values {
                    let ctid_term = Term::from_field_u64(ctid_field, ctid);
                    writer.delete_term(ctid_term);
                }
                ParadeWriterResponse::Ok
            }
        }
    }

    fn commit_with_writer(writer: &mut IndexWriter) -> ParadeWriterResponse {
        if let Err(e) = writer.prepare_commit() {
            pgrx::log!("error preparing commit to tantivy index: {e:?}");
            let msg = format!("error preparing commit to index: {e:?}");
            return ParadeWriterResponse::Error(msg);
        }

        if let Err(e) = writer.commit() {
            pgrx::log!("error committing to tantivy index: {e:?}");
            let msg = format!("error committing to index: {e:?}");
            return ParadeWriterResponse::Error(msg);
        }

        ParadeWriterResponse::Ok
    }

    fn commit(&mut self, index_directory_path: String) -> ParadeWriterResponse {
        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => Self::commit_with_writer(writer),
        }
    }

    fn vacuum(&mut self, index_directory_path: String) -> ParadeWriterResponse {
        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => {
                if let Err(e) = writer.garbage_collect_files().wait() {
                    let msg = format!("error vacuuming index: {e:?}");
                    return ParadeWriterResponse::Error(msg);
                }

                ParadeWriterResponse::Ok
            }
        }
    }

    fn drop_index(
        &mut self,
        index_directory_path: String,
        mut paths_to_delete: Vec<String>,
    ) -> ParadeWriterResponse {
        match self.writer(&index_directory_path) {
            Err(_) => {
                // The writer doesn't exist, but the files associated with the index might.
                // We'll proceed with the rest of the cleanup procedure.
            }
            Ok(writer) => {
                // Delete all Tantivy documents.

                // If the index directory folder has been deleted for some reason, the commands
                // below will fail and block the rest of the drop_index procedude. We'll just
                // skip the next two commands in this case, as there's nothing to delete if the
                // folder is somehow gone.
                if std::path::Path::new(&index_directory_path).exists() {
                    if let Err(e) = writer.delete_all_documents() {
                        let msg =
                            format!("error deleting tantivy documents during drop_index: {e:?}");
                        return ParadeWriterResponse::Error(msg);
                    }

                    // A commit is required after deleting the documents.
                    if let ParadeWriterResponse::Error(msg) = Self::commit_with_writer(writer) {
                        let msg = format!(
                            "error while commiting after tantivy deletion in drop_index: {msg}"
                        );
                        return ParadeWriterResponse::Error(msg);
                    }
                }
            }
        }

        // Remove the writer from the cache so that it is dropped.
        // We want to do this first so that the lockfile is released before deleting.
        // We'll manually call drop to make sure the lockfile is cleaned up.
        match self.writers.remove(&index_directory_path) {
            Some(writer) => std::mem::drop(writer),
            None => {
                let msg =
                    format!("no existing writer to drop for index at: {index_directory_path}");
                ParadeWriterResponse::Error(msg);
            }
        };

        // Filter out non-existent paths and sort: files first, then directories
        paths_to_delete.retain(|path| Path::new(path).exists());
        paths_to_delete.sort_by_key(|path| !Path::new(path).is_file());

        // Iterate through the sorted list and delete each path
        for path in paths_to_delete {
            let path_ref = Path::new(&path);
            if path_ref.is_file() {
                // Even though we've filtered out the files that supposedly don't exist above,
                // we can still see errors around files existing/not existing unexpectedly.
                // we'll just check again here to be safe.
                match path_ref.try_exists() {
                    Ok(true) => {
                        if let Err(e) = fs::remove_file(path_ref) {
                            let msg = format!(
                                "error deleting a file that exists during drop_index: {path} {e:?}"
                            );
                            return ParadeWriterResponse::Error(msg);
                        }
                    }
                    Ok(false) => {
                        // File does not exist, do nothing.
                    }
                    Err(e) => {
                        let msg = format!("error checking for file existence before deletion in drop_index: {e:?}");
                        return ParadeWriterResponse::Error(msg);
                    }
                }
            } else {
                match path_ref.try_exists() {
                    Ok(true) => {
                        if let Err(e) = fs::remove_dir_all(path_ref) {
                            let msg = format!(
                                "error deleting directory that exists during drop_index: {path}: {e:?}"
                            );
                            // There shouldn't be a problem with deleting a directory at this point, but empircally
                            // it has caused some issues during our integration tests. Since we haven't seen any
                            // problem with the folder still existing during regular use, we'll just log this error
                            // for now so our test suite doesn't break.
                            pgrx::log!("{msg}");
                        }
                    }
                    Ok(false) => {
                        // Directory does not exist, do nothing.
                    }
                    Err(e) => {
                        let msg = format!("error checking for directory existence before deletion in drop_index: {e:?}");
                        return ParadeWriterResponse::Error(msg);
                    }
                }
            }
        }

        ParadeWriterResponse::Ok
    }

    /// Shutdown the server. This should only ever be called by the shutdown bgworker.
    fn shutdown(&mut self) -> ParadeWriterResponse {
        self.should_exit = true;
        ParadeWriterResponse::Ok
    }
}
