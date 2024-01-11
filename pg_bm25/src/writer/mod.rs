use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    fs,
    path::Path,
};
use std::{marker::PhantomData, net::SocketAddr};
use tantivy::{schema::Field, Document, IndexWriter, Term};
use thiserror::Error;

use crate::{
    json::builder::{JsonBuilder, JsonBuilderValue},
    parade_index::index::ParadeIndex,
    parade_writer::transfer::{WriterTransferConsumer, WriterTransferProducer},
};

/// Possible actions to request of the ParadeWriterServer.
#[derive(Debug, Serialize, Deserialize)]
pub enum WriterRequest {
    Insert {
        index_directory_path: String,
        json_builder: JsonBuilder,
    },
    Delete {
        index_directory_path: String,
        field: Field,
        ctids: Vec<u64>,
    },
    DropIndex {
        index_directory_path: String,
        paths_to_delete: Vec<String>,
    },
    Commit {
        index_directory_path: String,
    },
    Vacuum {
        index_directory_path: String,
    },
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("could not parse response from writer server: {0}")]
    ResponseParseError(reqwest::Error),

    #[error("writer server responded with an error: {0}")]
    ServerError(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

pub struct Client<T: Serialize> {
    addr: std::net::SocketAddr,
    http: reqwest::blocking::Client,
    producer: Option<WriterTransferProducer<T>>,
    marker: PhantomData<T>,
}

#[derive(Serialize, Deserialize)]
enum ServerRequest<R: Serialize> {
    /// We have data in this request to send.
    Request(R),
    Transfer,
    Shutdown,
}

impl<T: Serialize> Client<T> {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            http: reqwest::blocking::Client::new(),
            producer: None,
            marker: PhantomData,
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn request(&self, request: T) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Request(request))
    }

    pub fn transfer(&mut self, request: T) -> Result<(), ClientError> {
        self.send_transfer(request)
    }

    fn send_request(&self, request: ServerRequest<T>) -> Result<(), ClientError> {
        let bytes = serde_json::to_vec(&request)?;
        let response = self.http.post(self.url()).body::<Vec<u8>>(bytes).send()?;

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => {
                let err = response
                    .text()
                    .map_err(|err| ClientError::ResponseParseError(err))?;
                Err(ClientError::ServerError(err))
            }
        }
    }

    fn send_transfer(&mut self, request: T) -> Result<(), ClientError> {
        if self.producer.is_none() {
            self.send_request(ServerRequest::Transfer)?;
            self.producer.replace(WriterTransferProducer::new()?);
        }
        self.producer.as_mut().unwrap().write_message(&request)?;
        Ok(())
    }

    fn stop_transfer(&mut self) {
        // Dropping the producer closes the named pipe file.
        self.producer.take();
    }

    /// Should only be called by shutdown background worker.
    pub fn stop_server(&self) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Shutdown)?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("only integer key fields are supported for parade index")]
    InvalidKeyField,

    #[error("error binding writer server to address: {0}")]
    AddressBindFailed(String),

    #[error("couldn't get writer for index {0}: {1}")]
    GetWriterFailed(String, String),

    #[error("writer server must not bind to unix socket, attemped: {0}")]
    UnixSocketBindAttempt(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),
}

pub struct Server<'a, T: Serialize + DeserializeOwned + 'a, H: WriterHandler<T>> {
    addr: std::net::SocketAddr,
    http: tiny_http::Server,
    should_exit: bool,
    handler: RefCell<H>,
    consumer: RefCell<WriterTransferConsumer<T>>,
    marker: PhantomData<&'a T>,
}

impl<'a, T: Serialize + DeserializeOwned + 'a, H: WriterHandler<T>> Server<'a, T, H> {
    fn new(handler: H) -> Result<Self, ServerError> {
        let http = tiny_http::Server::http("0.0.0.0:0")
            .map_err(|err| ServerError::AddressBindFailed(err.to_string()))?;

        let addr = match http.server_addr() {
            tiny_http::ListenAddr::IP(addr) => addr,
            // It's not clear when tiny_http would choose to use a Unix socket address,
            // but we have to handle the enum variant, so we'll consider this outcome
            // an irrecovereable error, although its not expected to happen.
            tiny_http::ListenAddr::Unix(addr) => {
                return Err(ServerError::UnixSocketBindAttempt(format!("{addr:?}")))
            }
        };

        Ok(Self {
            addr,
            http,
            handler: RefCell::new(handler),
            should_exit: false,
            consumer: RefCell::new(WriterTransferConsumer::new()),
            marker: PhantomData,
        })
    }

    fn listen_transfer(&self) -> Result<(), ServerError> {
        for incoming in self.consumer.borrow_mut().read_stream() {
            self.handler.borrow_mut().handle(incoming?)?;
        }
        Ok(())
    }

    fn listen_request(&mut self) -> Result<(), ServerError> {
        for mut incoming in self.http.incoming_requests() {
            let reader = incoming.as_reader();
            let request: Result<ServerRequest<T>, ServerError> =
                serde_json::from_reader(reader).map_err(ServerError::SerdeError);

            let mut intiate_transfer = false;

            let response = match request {
                Ok(req) => match req {
                    ServerRequest::Shutdown => return Ok(()),
                    ServerRequest::Transfer => {
                        intiate_transfer = true;
                        Ok(()) // We must respond with OK before initiating the transfer.
                    }
                    ServerRequest::Request(writer_request) => {
                        self.handler.borrow_mut().handle(writer_request)
                    }
                },
                Err(err) => Err(err),
            };

            if let Err(err) = match response {
                Ok(()) => incoming.respond(tiny_http::Response::empty(200)),
                Err(err) => incoming.respond(
                    tiny_http::Response::from_string(err.to_string()).with_status_code(500),
                ),
            } {
                pgrx::log!("writer server failed to respond to client: {err:?}");
                continue; // Ignore any pending transfer initiation if the client is broken.
            }

            if intiate_transfer {
                self.listen_transfer()?
            }
        }

        unreachable!("server should never stop listening");
    }
}

pub struct Writer {
    /// Map of index directory path to Tantivy writer instance.
    tantivy_writers: HashMap<String, tantivy::IndexWriter>,
}

impl Writer {
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
            let ctid_term = tantivy::Term::from_field_u64(ctid_field.clone(), ctid.clone());
            writer.delete_term(ctid_term);
        }
        Ok(())
    }

    fn commit(&mut self, index_directory_path: &str) -> Result<(), ServerError> {
        let writer = self.get_writer(index_directory_path)?;
        writer.prepare_commit()?;
        writer.commit()?;
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
                // TODO: COMMIT HERE!
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

pub trait WriterHandler<T: Serialize> {
    fn handle(&mut self, request: T) -> Result<(), ServerError> {
        Ok(())
    }
}

impl WriterHandler<WriterRequest> for Writer {
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
            WriterRequest::Commit {
                index_directory_path,
            } => self.commit(&index_directory_path),
            WriterRequest::Vacuum {
                index_directory_path,
            } => self.vacuum(&index_directory_path),
        }
    }
}
