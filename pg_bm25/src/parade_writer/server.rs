use crate::json::builder::JsonBuilderValue;
use crate::parade_writer::{ParadeWriterRequest, ParadeWriterResponse};
use crate::{json::builder::JsonBuilder, parade_index::index::ParadeIndex};
use std::collections::{hash_map::Entry::Vacant, HashMap};
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
            ParadeWriterRequest::DropIndex(index_directory_path) => {
                self.drop_index(&index_directory_path)
            }
        }
    }

    /// Check the writer server cache for an existing IndexWriter. If it does not exist,
    /// then retrieve the ParadeIndex and use it to create a new IndexWriter, caching it.
    fn writer(&mut self, index_directory_path: &str) -> Result<&mut IndexWriter, String> {
        if let Vacant(entry) = self.writers.entry(index_directory_path.to_string()) {
            if let Err(e) = ParadeIndex::writer(&index_directory_path)
                .and_then(|writer| Ok(entry.insert(writer)))
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
            Some(JsonBuilderValue::i16(value)) => value.clone() as i64,
            Some(JsonBuilderValue::i32(value)) => value.clone() as i64,
            Some(JsonBuilderValue::i64(value)) => value.clone() as i64,
            Some(JsonBuilderValue::u32(value)) => value.clone() as i64,
            Some(JsonBuilderValue::u64(value)) => value.clone() as i64,
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
                let key_term = Term::from_field_i64(key_field, key_value.clone());
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

    fn commit(&mut self, index_directory_path: String) -> ParadeWriterResponse {
        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => {
                if let Err(e) = writer.prepare_commit() {
                    let msg = format!("error preparing commit to index: {e:?}");
                    return ParadeWriterResponse::Error(msg);
                }

                if let Err(e) = writer.commit() {
                    let msg = format!("error committing to index: {e:?}");
                    return ParadeWriterResponse::Error(msg);
                }

                ParadeWriterResponse::Ok
            }
        }
    }

    fn drop_index(&mut self, index_directory_path: &str) -> ParadeWriterResponse {
        if let Err(e) = ParadeIndex::delete_index_directory(index_directory_path) {
            ParadeWriterResponse::Error(e.to_string())
        } else {
            // Remove the write from the cache so that it is dropped.
            self.writers.remove(index_directory_path);
            ParadeWriterResponse::Ok
        }
    }

    /// Shutdown the server. This should only ever be called by the shutdown bgworker.
    fn shutdown(&mut self) -> ParadeWriterResponse {
        self.should_exit = true;
        ParadeWriterResponse::Ok
    }
}
