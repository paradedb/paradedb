use core::fmt;

use pgrx::PGRXSharedMemory;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug)]
pub enum WriterError {
    ServerUnixPortError,
    ServerBindError,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ParadeWriterRequestError {
    JsonParseError(String),
    IoError(String),
    WriterNotInitialized(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ParadeWriterResponseError {
    IoError(String),
    Utf8Error(String),
    JsonParseError(String),
    UnsupportedRequestType,
}

unsafe impl PGRXSharedMemory for WriterError {}

impl fmt::Display for ParadeWriterRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let serialized = serde_json::to_string(self)
            .unwrap_or_else(|_| "Error serializing ParadeWriterRequestError".to_string());
        write!(f, "{}", serialized)
    }
}

impl std::error::Error for ParadeWriterRequestError {}

impl fmt::Display for ParadeWriterResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let serialized = serde_json::to_string(self)
            .unwrap_or_else(|_| "Error serializing ParadeWriterResponseError".to_string());
        write!(f, "{}", serialized)
    }
}

impl std::error::Error for ParadeWriterResponseError {}
