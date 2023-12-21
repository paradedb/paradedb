pub mod client;
pub mod server;

use crate::json::builder::JsonBuilder;
pub use client::ParadeWriterClient;
use pgrx::PGRXSharedMemory;
use serde::{Deserialize, Serialize};
pub use server::ParadeWriterServer;
use tantivy::schema::Field;

/// Possible actions to request of the ParadeWriterServer.
#[derive(Debug, Serialize, Deserialize)]
pub enum ParadeWriterRequest {
    /// index_directory_path, json_builder.
    Insert(String, JsonBuilder),
    /// index_directory_path, vector of ctid values.
    Delete(String, Field, Vec<u64>),
    /// index_directory_path, json_builder.
    DropIndex(String),
    /// index_directory_path.
    Commit(String),
    /// should only be called by shutdown bgworker.
    Shutdown,
}

/// Possible responses for the ParadeWriterServer.
/// The ParadeWriterServer must not every panic, because it doesn't have
/// a reliable way to recover, and observability into it is very difficult.
/// If it does need to error, it should return one of these error types to the client.
/// To ease debugging, we should tr
#[derive(Serialize, Deserialize, Debug)]
pub enum ParadeWriterResponse {
    Ok,
    Error(String),
}

// We're using the From/TryFrom traits to handle serialization/deserialization.
// These should call out to serde-based serialization functions.

impl From<ParadeWriterRequest> for Vec<u8> {
    fn from(parade_writer_request: ParadeWriterRequest) -> Self {
        serde_json::to_vec(&parade_writer_request).unwrap()
    }
}

impl TryFrom<&mut tiny_http::Request> for ParadeWriterRequest {
    type Error = String;

    fn try_from(request: &mut tiny_http::Request) -> Result<Self, Self::Error> {
        let reader = request.as_reader();
        serde_json::from_reader(reader).map_err(|e| e.to_string())
    }
}

impl From<ParadeWriterResponse> for Vec<u8> {
    fn from(value: ParadeWriterResponse) -> Self {
        serde_json::to_vec(&value).unwrap()
    }
}

impl TryFrom<&[u8]> for ParadeWriterResponse {
    type Error = String;

    fn try_from(value: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(value).map_err(|e| e.to_string())
    }
}

/// We specifically define a WriterInitError for errors that can occur before
/// we've started the server. Any of these will leave  pg_bm25 in a completely
/// broken state, so we should do our best to abort Postgres startup in that case.
#[derive(Copy, Clone, Debug)]
pub enum WriterInitError {
    ServerUnixPortError,
    ServerBindError,
}

unsafe impl PGRXSharedMemory for WriterInitError {}
