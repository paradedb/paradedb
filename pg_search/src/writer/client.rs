// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::globals::WRITER_GLOBAL;

use super::{transfer::WriterTransferProducer, ServerRequest, WriterClient};
use serde::Serialize;
use std::{marker::PhantomData, net::SocketAddr, panic, path::Path};
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

    pub fn from_global() -> Self {
        let lock = panic::catch_unwind(|| WRITER_GLOBAL.share());

        let addr = match lock {
            Ok(lock) => lock.addr(),
            Err(_) => {
                panic!("Could not get lock on writer. Have you added the extension to the shared preload library list?");
            }
        };

        Self::new(addr)
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn send_request(&mut self, request: ServerRequest<T>) -> Result<(), ClientError> {
        // If there is an open pending transfer, stop it so that we can continue
        // with more requests.
        self.stop_transfer();
        let bytes = serde_json::to_string(&request).unwrap();
        let response = self
            .http
            .post(self.url())
            .body::<Vec<u8>>(bytes.into_bytes())
            .send()?;

        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => {
                let err = response.text().map_err(ClientError::ResponseParse)?;
                Err(ClientError::ServerError(err))
            }
        }
    }

    fn send_transfer<P: AsRef<Path>>(
        &mut self,
        pipe_path: P,
        request: T,
    ) -> Result<(), ClientError> {
        if self.producer.is_none() {
            // Send a request to open a transfer to the server.
            self.send_request(ServerRequest::Transfer(
                pipe_path.as_ref().display().to_string(),
            ))?;
            // Store a new transfer producer in the client state.
            self.producer
                .replace(WriterTransferProducer::new(pipe_path)?);
        }

        // There is an existing producer in client state, use it to send the request.
        self.producer
            .as_mut()
            .unwrap()
            .write_message(&request)
            .map_err(|err| {
                anyhow::anyhow!(
                    "unexpected error while transfering data to pg_search writer server... please check your postgres logs for details: {err}"
                )
            })?;
        Ok(())
    }

    /// Stop a data pipe transfer. Must be called when the transfer is done, or
    /// the client + server will both hang forever.
    ///
    /// With insert transactions, it's tricky to know when the transfer is
    /// completely done. Best practice is to call this both during the end of
    /// transaction callback, as well as before every send_request.
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

impl<T: Serialize> WriterClient<T> for Client<T> {
    fn request(&mut self, request: T) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Request(request))
    }

    fn transfer<P: AsRef<Path>>(&mut self, pipe_path: P, request: T) -> Result<(), ClientError> {
        self.send_transfer(pipe_path, request)
    }
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("could not parse response from writer server: {0}")]
    ResponseParse(reqwest::Error),

    #[error("writer server responded with an error: {0}")]
    ServerError(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("unexpected error (anyhow): {0}")]
    Anyhow(#[from] anyhow::Error),
}

// #[cfg(test)]
// mod tests {
//     use crate::fixtures::*;
//     use crate::writer::{Client, Server, WriterClient, WriterRequest};
//     use rstest::*;
//     use std::thread;

//     #[rstest]
//     #[case::insert_request(WriterRequest::Insert {
//         directory: mock_dir().writer_dir,
//         document: simple_doc(simple_schema(default_fields())),
//     })]
//     #[case::commit_request(WriterRequest::Commit { directory: mock_dir().writer_dir })]
//     #[case::abort_request(WriterRequest::Abort {directory: mock_dir().writer_dir})]
//     #[case::vacuum_request(WriterRequest::Vacuum { directory: mock_dir().writer_dir })]
//     #[case::drop_index_request(WriterRequest::DropIndex { directory: mock_dir().writer_dir })]
//     /// Test request serialization and transfer between client and server.
//     fn test_client_request(#[case] request: WriterRequest) {
//         // Create a handler that will test that the received request is the same as sent.
//         let request_clone = request.clone();
//         let handler = TestHandler::new(move |req: WriterRequest| assert_eq!(&req, &request_clone));
//         let mut server = Server::new(handler).unwrap();
//         let addr = server.addr();

//         // Start the server in a new thread, as it blocks once started.
//         thread::spawn(move || {
//             server.start().unwrap();
//         });

//         let mut client: Client<WriterRequest> = Client::new(addr);
//         client.request(request.clone()).unwrap();

//         // The server must be stopped, or this test will not finish.
//         client.stop_server().unwrap();
//     }
// }
