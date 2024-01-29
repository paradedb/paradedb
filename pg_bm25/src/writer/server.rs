use super::{Handler, IndexError, ServerRequest};
use crate::writer::transfer;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io;
use std::marker::PhantomData;
use std::path::Path;
use std::{cell::RefCell, io::Cursor};
use thiserror::Error;
use tracing::{error, info};

/// A generic server for receiving requests and transfers from a client.
pub struct Server<'a, T: DeserializeOwned, H: Handler<T>> {
    addr: std::net::SocketAddr,
    http: tiny_http::Server,
    handler: RefCell<H>,
    marker: PhantomData<&'a T>,
}

impl<'a, T: Serialize + DeserializeOwned + 'a, H: Handler<T>> Server<'a, T, H> {
    pub fn new(handler: H) -> Result<Self, ServerError> {
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
            marker: PhantomData,
        })
    }

    pub fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    pub fn start(&mut self) -> Result<(), ServerError> {
        self.listen_request()
    }

    fn listen_transfer<P: AsRef<Path>>(&self, pipe_path: P) -> Result<(), ServerError> {
        // Our consumer will receive messages suitable for our handler.
        for incoming in transfer::read_stream::<T, P>(pipe_path)? {
            // pgrx::log!(
            //     "INCOMING: {:#?}",
            //     serde_json::to_string(&incoming.as_ref().unwrap())
            // );
            let incoming = incoming.map_err(|err| ServerError::Unexpected(err))?;
            self.handler.borrow_mut().handle(incoming)?;
        }

        Ok(())
    }

    fn response_ok() -> tiny_http::Response<io::Empty> {
        tiny_http::Response::empty(200)
    }

    fn response_err(err: ServerError) -> tiny_http::Response<Cursor<Vec<u8>>> {
        tiny_http::Response::from_string(err.to_string()).with_status_code(500)
    }

    fn listen_request(&mut self) -> Result<(), ServerError> {
        info!("listening to incoming requests at {:?}", self.addr);
        for mut incoming in self.http.incoming_requests() {
            let reader = incoming.as_reader();
            let request: Result<ServerRequest<T>, ServerError> = bincode::deserialize_from(reader)
                .map_err(|err| ServerError::Unexpected(err.into()));

            match request {
                Ok(req) => match req {
                    ServerRequest::Shutdown => {
                        if let Err(err) = incoming.respond(Self::response_ok()) {
                            error!("server error responding to shutdown: {err}");
                        }
                        return Ok(());
                    }
                    ServerRequest::Transfer(pipe_path) => {
                        // We must respond with OK before initiating the transfer.
                        if let Err(err) = incoming.respond(Self::response_ok()) {
                            error!("server error responding to transfer: {err}");
                        } else {
                            if let Err(err) = self.listen_transfer(pipe_path) {
                                error!("error listening to transfer: {err}")
                            }
                        }
                    }
                    ServerRequest::Request(req) => {
                        if let Err(err) = self.handler.borrow_mut().handle(req) {
                            if let Err(err) = incoming.respond(Self::response_err(err)) {
                                error!("server error responding to handler error: {err}");
                            }
                        } else {
                            if let Err(err) = incoming.respond(Self::response_ok()) {
                                error!("server error responding to handler success: {err}")
                            }
                        }
                    }
                },
                Err(err) => {
                    if let Err(err) = incoming.respond(Self::response_err(err)) {
                        error!("server error responding to client on deserialize error: {err}");
                    }
                }
            };
        }

        unreachable!("server should never stop listening");
    }
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("couldn't open the consumer pipe file: {0}")]
    OpenPipeFile(std::io::Error),

    #[error("error binding writer server to address: {0}")]
    AddressBindFailed(String),

    #[error("writer server must not bind to unix socket, attemped: {0}")]
    UnixSocketBindAttempt(String),

    #[error(transparent)]
    WriterError(#[from] IndexError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error("unexpected error: {0}")]
    Unexpected(#[from] Box<dyn std::error::Error>),
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use tantivy::{schema::Schema, Document, Index, IndexSettings, IndexSortByField, Order};
    use tantivy_common::BinarySerializable;

    #[test]
    fn test_tantivy_commit_bug() {
        let serialized_schema = r#"
      [{"name":"category","type":"text","options":{"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"stored":true,"fast":false}},{"name":"description","type":"text","options":{"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"stored":true,"fast":false}},{"name":"rating","type":"i64","options":{"indexed":true,"fieldnorms":false,"fast":true,"stored":true}},{"name":"in_stock","type":"bool","options":{"indexed":true,"fieldnorms":false,"fast":true,"stored":true}},{"name":"metadata","type":"json_object","options":{"stored":true,"indexing":{"record":"position","fieldnorms":true,"tokenizer":"default"},"fast":false,"expand_dots_enabled":true}},{"name":"id","type":"i64","options":{"indexed":true,"fieldnorms":true,"fast":true,"stored":true}},{"name":"ctid","type":"u64","options":{"indexed":true,"fieldnorms":true,"fast":true,"stored":true}}]
    "#;

        let schema: Schema = serde_json::from_str(&serialized_schema).unwrap();
        let settings = IndexSettings {
            sort_by_field: Some(IndexSortByField {
                field: "id".into(),
                order: Order::Asc,
            }),
            ..Default::default()
        };

        let temp_dir = tempfile::Builder::new().tempdir().unwrap();

        let index = Index::builder()
            .schema(schema)
            .settings(settings)
            .create_in_dir(&temp_dir.path())
            .unwrap();

        let mut writer = index.writer(500_000_000).unwrap();

        // This is a string representation of the document bytes that I am sending through IPC.
        let document_bytes: Vec<u8> = serde_json::from_str("[135,5,0,0,0,2,1,0,0,0,0,0,0,0,1,0,0,0,0,152,69,114,103,111,110,111,109,105,99,32,109,101,116,97,108,32,107,101,121,98,111,97,114,100,2,0,0,0,2,4,0,0,0,0,0,0,0,0,0,0,0,0,139,69,108,101,99,116,114,111,110,105,99,115,3,0,0,0,9,1,4,0,0,0,8,123,34,99,111,108,111,114,34,58,34,83,105,108,118,101,114,34,44,34,108,111,99,97,116,105,111,110,34,58,34,85,110,105,116,101,100,32,83,116,97,116,101,115,34,125,5,0,0,0,1,1,0,0,0,0,0,0,0]").unwrap();

        let document_from_bytes: Document =
            BinarySerializable::deserialize(&mut Cursor::new(document_bytes)).unwrap();

        // This is a json representation of the above that I'm including here for readability.
        // This was generated with `println!(serde_json::to_string(document_from_bytes).unwrap())`.
        let document_json = r#"
            {"field_values":[]}
        "#;

        let document_from_json: Document = serde_json::from_str(document_json).unwrap();

        // // To prove that the document_json and the document_from_bytes represent the same Document,
        // // we assert their equality here. This is expected to pass.
        // assert_eq!(
        //     document_json.trim(),
        //     serde_json::to_string(&document_from_bytes).unwrap().trim()
        // );

        writer.add_document(document_from_json).unwrap();

        // We expect an error here on commit: ErrorInThread("Any { .. }")
        writer.commit().unwrap();
    }
}
