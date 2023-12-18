use std::{error::Error, net::SocketAddr};

use pgrx::{log, PGRXSharedMemory};

use crate::parade_writer::io::ParadeWriterResponse;

use self::{
    error::{ParadeWriterRequestError, WriterError},
    io::ParadeWriterRequest,
};

pub mod error;
pub mod io;

#[allow(unused_variables)]
#[derive(Copy, Clone, Default)]
pub struct ParadeWriter {
    addr: Option<SocketAddr>,
    error: Option<WriterError>,
}

impl ParadeWriter {
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = Some(addr);
    }

    pub fn set_error(&mut self, err: WriterError) {
        self.error = Some(err);
    }

    fn send_request(
        &self,
        request: ParadeWriterRequest,
    ) -> Result<ParadeWriterResponse, Box<dyn Error>> {
        let addr = if self.addr.is_none() {
            match request {
                ParadeWriterRequest::Shutdown => {
                    // The insert worker hasn't started its server yet.
                    // We won't send the shutdown request,but it is up to the insert worker
                    // to handle this case by checking for SIGTERM right before starting its server.
                    log!("pg_bm25 shutdown worker skipped sending signal to insert worker");
                    return Ok(ParadeWriterResponse::ShutdownOk);
                }
                req => {
                    return Err(Box::new(ParadeWriterRequestError::WriterNotInitialized(
                        format!(
                            "pg_bm25 writer not yet initialized, but received request: {req:?}"
                        ),
                    )))
                }
            }
        } else {
            self.addr.unwrap()
        };

        let uri = &request.uri();
        let bytes: Vec<u8> = request.into();
        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&format!("http://{addr}{uri}"))
            .body(bytes)
            .send()?;
        let response_body = response.bytes()?;
        ParadeWriterResponse::try_from(response_body.to_vec().as_slice()).map_err(|e| e.into())
    }

    pub fn shutdown(&self) -> Result<(), Box<dyn Error>> {
        self.send_request(ParadeWriterRequest::Shutdown)?;
        Ok(())
    }
}

unsafe impl PGRXSharedMemory for ParadeWriter {}
