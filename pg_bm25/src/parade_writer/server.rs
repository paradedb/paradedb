use crate::parade_writer::{ParadeWriterRequest, ParadeWriterResponse};
use crate::{json::builder::JsonBuilder, parade_index::index::ParadeIndex};
use pgrx::log;
use std::collections::{hash_map::Entry::Vacant, HashMap};
use tantivy::{Document, IndexWriter};

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
            ParadeWriterRequest::Insert(index_directory_path, json_builder) => {
                self.insert(index_directory_path, json_builder)
            }
            t => {
                log!("server received unimplemented type: {t:?}");
                unimplemented!("server received unimplemented type: {t:?}");
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
        match self.writer(&index_directory_path) {
            Err(e) => ParadeWriterResponse::Error(e.to_string()),
            Ok(writer) => {
                // Add each of the fields to the Tantivy document.
                let mut doc: Document = Document::new();
                for (field, value) in json_builder.values.iter() {
                    value.add_to_tantivy_doc(&mut doc, field);
                }

                // Add the Tantivy document to the index.
                if let Err(e) = writer.add_document(doc) {
                    return ParadeWriterResponse::Error(e.to_string());
                }

                if let Err(e) = writer.prepare_commit() {
                    return ParadeWriterResponse::Error(e.to_string());
                }

                if let Err(e) = writer.commit() {
                    return ParadeWriterResponse::Error(e.to_string());
                }

                ParadeWriterResponse::Ok
            }
        }
    }

    // fn delete(
    //     &mut self,
    //     index_directory_path: &str,
    //     json_builder,
    // )

    /// Shutdown the server. This should only ever be called by the shutdown bgworker.
    fn shutdown(&mut self) -> ParadeWriterResponse {
        self.should_exit = true;
        ParadeWriterResponse::Ok
    }
}
