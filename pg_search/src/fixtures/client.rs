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

use crate::writer::{ClientError, Handler, Writer, WriterClient, WriterRequest};
use std::sync::{Arc, Mutex};

pub struct TestClient {
    writer: Writer,
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
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
