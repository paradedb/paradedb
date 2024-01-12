mod client;
mod index;
mod server;
mod transfer;

use crate::json::builder::JsonBuilder;
pub use client::{Client, ClientError};
pub use index::Writer;
use serde::{Deserialize, Serialize};
pub use server::{Server, ServerError};
use tantivy::schema::Field;

// A layer of the client-server request structure that handles
// details about the action to be performed by the index writer.
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
    Commit,
    Vacuum {
        index_directory_path: String,
    },
}

// A layer of the client-server request structure that handles
// details around actions the server should perform.
#[derive(Serialize, Deserialize)]
enum ServerRequest<R: Serialize> {
    /// Request with payload.
    Request(R),
    /// Initiate a data transfer using the pipe path given.
    Transfer(String),
    /// Close the writer server, should only be called by
    /// shutdown background worker.
    Shutdown,
}

/// This trait is the interface that binds the writer to the server.
/// The two systems are otherwise decoupled, so they can be tested
/// and re-used independently.
pub trait Handler<T: Serialize> {
    fn handle(&mut self, request: T) -> Result<(), ServerError> {
        Ok(())
    }
}
