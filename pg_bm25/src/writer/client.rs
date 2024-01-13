use crate::WRITER_STATUS;

use super::{transfer::WriterTransferProducer, IndexEntry, ServerRequest};
use serde::Serialize;
use std::{marker::PhantomData, net::SocketAddr};
use thiserror::Error;

pub struct Client<T: Serialize> {
    addr: std::net::SocketAddr,
    http: reqwest::blocking::Client,
    producer: Option<WriterTransferProducer<T>>,
    marker: PhantomData<T>,
}

/// A generic client for communication with background server.
/// The client has two functions, "request" and "transfer".

/// "request" sends a synchronous request and waits for a response.

/// "transfer" sends a request, and then opens a data pipe to the backend.
/// This is useful for transfering large volumes of data, where "request"
/// has too much overhead to be called over and over.

/// A transfer requires exclusive access to the background server, so
/// during a transfer, other connections will block and wait for the
/// background server to become available again.
impl<T: Serialize> Client<T> {
    pub fn new(addr: SocketAddr) -> Self {
        // Some server processes, like creating a index, can take a long time.
        // Because the server is blocking/single-threaded, clients should wait
        // as long as they need to for their turn to use the server.
        let http = reqwest::blocking::ClientBuilder::new()
            .timeout(None)
            .build()
            .expect("error building http client");

        Self {
            addr,
            http,
            producer: None,
            marker: PhantomData,
        }
    }

    pub fn from_writer_addr() -> Self {
        let addr = WRITER_STATUS.share().addr();
        Self::new(addr)
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn request(&mut self, request: T) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Request(request))
    }

    pub fn transfer(&mut self, request: T) -> Result<(), ClientError> {
        self.send_transfer(request)
    }

    fn send_request(&mut self, request: ServerRequest<T>) -> Result<(), ClientError> {
        // If there is an open pending transfer, stop it so that we can continue
        // with more requests.
        self.stop_transfer();
        let bytes = serde_json::to_vec(&request)?;
        pgrx::log!(
            "sending request {:?}",
            serde_json::to_string_pretty(&request)
        );
        let response = self.http.post(self.url()).body::<Vec<u8>>(bytes).send()?;
        pgrx::log!(
            "received response {:?}",
            serde_json::to_string_pretty(&request)
        );

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => {
                let err = response.text().map_err(ClientError::ResponseParseError)?;
                Err(ClientError::ServerError(err))
            }
        }
    }

    fn send_transfer(&mut self, request: T) -> Result<(), ClientError> {
        if self.producer.is_none() {
            let pipe_path = WriterTransferProducer::<IndexEntry>::pipe_path()?
                .display()
                .to_string();
            self.send_request(ServerRequest::Transfer(pipe_path))?;
            self.producer.replace(WriterTransferProducer::new()?);
        }
        self.producer.as_mut().unwrap().write_message(&request)?;
        Ok(())
    }

    /// Stop a data pipe transfer. Must be called when the transfer is done, or
    /// the client + server will both hang forever.
    ///
    /// With contexts like inserting, it's tricky to know when the transfer is
    /// completely done. You can't necessarily wait until the end of the transaction,
    /// because there may be more writer operations (delete etc.) in the same transaction.
    /// Best practice is to call this both during the end of transaction callback, as well
    /// as before every send_request.
    fn stop_transfer(&mut self) {
        // Dropping the producer closes the named pipe file.
        self.producer.take();
    }

    /// Should only be called by shutdown background worker.
    pub fn stop_server(&mut self) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Shutdown)?;
        Ok(())
    }
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
