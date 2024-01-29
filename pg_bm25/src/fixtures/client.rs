use crate::writer::{ClientError, Handler, Writer, WriterClient, WriterRequest};
use std::sync::{Arc, Mutex};

pub struct TestClient {
    writer: Writer,
}

impl TestClient {
    pub fn new() -> Self {
        Self {
            writer: Writer::new(),
        }
    }

    pub fn new_arc() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new()))
    }
}

impl WriterClient<WriterRequest> for TestClient {
    fn request(&mut self, request: WriterRequest) -> Result<(), ClientError> {
        // Serialize the data to emulate the real transfer process.
        let serialized_request = bincode::serialize(&request).unwrap();
        let deserialized_request: WriterRequest =
            bincode::deserialize(&serialized_request).unwrap();
        self.writer
            .handle(deserialized_request)
            .map_err(|err| ClientError::ServerError(err.to_string()))
    }

    fn transfer<P: AsRef<std::path::Path>>(
        &mut self,
        _pipe_path: P,
        request: WriterRequest,
    ) -> Result<(), ClientError> {
        // Serialize the data to emulate the real transfer process.
        let serialized_request = bincode::serialize(&request).unwrap();
        let deserialized_request: WriterRequest =
            bincode::deserialize(&serialized_request).unwrap();
        self.request(deserialized_request)
    }
}
