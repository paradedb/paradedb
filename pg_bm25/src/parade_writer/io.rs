use crate::parade_index::index::ParadeIndexKey;
use serde::{Deserialize, Serialize};

use super::error::ParadeWriterResponseError;

#[derive(Debug, Serialize, Deserialize)]
pub enum ParadeWriterRequest {
    Insert(serde_json::Value),
    Delete(String, ParadeIndexKey),
    DropIndex(String),
    Shutdown,
}

impl ParadeWriterRequest {
    pub fn uri(&self) -> String {
        match self {
            ParadeWriterRequest::Insert(_) => "/INSERT",
            ParadeWriterRequest::Delete(_, _) => "/DELETE",
            ParadeWriterRequest::DropIndex(_) => "/DROP_INDEX",
            ParadeWriterRequest::Shutdown => "/SHUTDOWN",
        }
        .into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ParadeWriterResponse {
    ShutdownOk,
    ShutdownError(ParadeWriterResponseError),
    RequestParseError(ParadeWriterResponseError),
}

impl From<ParadeWriterRequest> for Vec<u8> {
    fn from(parade_writer_request: ParadeWriterRequest) -> Self {
        serde_json::to_vec(&parade_writer_request).unwrap()
    }
}

impl TryFrom<&mut tiny_http::Request> for ParadeWriterRequest {
    type Error = ParadeWriterResponseError;

    fn try_from(request: &mut tiny_http::Request) -> Result<Self, Self::Error> {
        // Read the entire body
        let mut body = String::new();
        request
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|e| ParadeWriterResponseError::IoError(e.to_string()))?;

        // Match the URL and deserialize appropriately
        match request.url() {
            "/INSERT" => {
                let value: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| ParadeWriterResponseError::JsonParseError(e.to_string()))?;
                Ok(ParadeWriterRequest::Insert(value))
            }
            "/DELETE" => {
                // Assuming that the body for DELETE is a JSON object with specific fields
                let delete_request: (String, ParadeIndexKey) = serde_json::from_str(&body)
                    .map_err(|e| ParadeWriterResponseError::JsonParseError(e.to_string()))?;
                Ok(ParadeWriterRequest::Delete(
                    delete_request.0,
                    delete_request.1,
                ))
            }
            "/DROP_INDEX" => {
                let index_name: String = serde_json::from_str(&body)
                    .map_err(|e| ParadeWriterResponseError::JsonParseError(e.to_string()))?;
                Ok(ParadeWriterRequest::DropIndex(index_name))
            }
            "/SHUTDOWN" => Ok(ParadeWriterRequest::Shutdown),
            _ => Err(ParadeWriterResponseError::UnsupportedRequestType),
        }
    }
}

impl From<ParadeWriterResponse> for Vec<u8> {
    fn from(value: ParadeWriterResponse) -> Self {
        serde_json::to_vec(&value).unwrap()
    }
}

impl TryFrom<&[u8]> for ParadeWriterResponse {
    type Error = serde_json::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(value)
    }
}
