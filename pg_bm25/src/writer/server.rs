use crate::writer::transfer;

use super::{Handler, ServerRequest};
use serde::{de::DeserializeOwned, Serialize};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::path::Path;
use thiserror::Error;

/// A generic server for receiving requests and transfers from a client.
pub struct Server<'a, T: Serialize + DeserializeOwned + 'a, H: Handler<T>> {
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
        for incoming in transfer::read_stream(pipe_path)? {
            self.handler.borrow_mut().handle(incoming?)?;
        }
        Ok(())
    }

    fn listen_request(&mut self) -> Result<(), ServerError> {
        pgrx::log!("listening to incoming requests at {:?}", self.addr);
        for mut incoming in self.http.incoming_requests() {
            let reader = incoming.as_reader();
            let request: Result<ServerRequest<T>, ServerError> =
                serde_json::from_reader(reader).map_err(ServerError::SerdeError);

            // A flag to tell us after we've sent the response that the client has
            // a data transfer to send us. The response must be returned before the transfer.
            let mut transfer_pipe_path: Option<String> = None;

            let response = match request {
                Ok(req) => match req {
                    ServerRequest::Shutdown => return Ok(()),
                    ServerRequest::Transfer(pipe_path) => {
                        transfer_pipe_path.replace(pipe_path);
                        Ok(()) // We must respond with OK before initiating the transfer.
                    }
                    ServerRequest::Request(writer_request) => {
                        self.handler.borrow_mut().handle(writer_request)
                    }
                },
                Err(err) => Err(err),
            };

            // Try to respond to the client. This could fail if the client has disconnected.
            if let Err(err) = match response {
                Ok(()) => incoming.respond(tiny_http::Response::empty(200)),
                Err(err) => incoming.respond(
                    tiny_http::Response::from_string(err.to_string()).with_status_code(500),
                ),
            } {
                pgrx::log!("writer server failed to respond to client: {err:?}");
                continue; // Ignore any pending transfer initiation if the client is broken.
            }

            // If this was a transfer request, we'll start listening for data.
            if let Some(pipe_path) = transfer_pipe_path {
                self.listen_transfer(pipe_path)?
            }
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
