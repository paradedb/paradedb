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

use crate::writer::ServerError;
use interprocess::os::unix::fifo_file;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::marker::PhantomData;
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tracing::warn;

#[derive(Deserialize, Serialize)]
pub enum WriterTransferMessage<T: Serialize> {
    Data(T),
    Done,
}
pub struct WriterTransferMessageIterator<R, T> {
    stream: BufReader<R>,
    phantom: std::marker::PhantomData<T>,
}

impl<R, T> WriterTransferMessageIterator<R, T>
where
    R: Read,
{
    pub fn new(reader: R) -> Self {
        WriterTransferMessageIterator {
            stream: BufReader::new(reader),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<R, T> Iterator for WriterTransferMessageIterator<R, T>
where
    R: Read,
    T: DeserializeOwned + Serialize,
{
    type Item = bincode::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match bincode::deserialize_from(&mut self.stream) {
            Err(err) => Some(Err(err)),
            Ok(WriterTransferMessage::Done) => None,
            Ok(WriterTransferMessage::Data(data)) => Some(Ok(data)),
        }
    }
}

pub struct WriterTransferProducer<T: Serialize> {
    pipe: File,
    pipe_path: PathBuf,
    marker: PhantomData<T>,
}

impl<T: Serialize> WriterTransferProducer<T> {
    pub fn new<P: AsRef<Path>>(pipe_path: P) -> std::io::Result<Self> {
        // It's important that this process is the "owner" of the named pipe file path.
        // We'll remove any existing pipe_path, and connect to the first producer
        // process who creates a new one.
        Self::delete_named_pipe_file(pipe_path.as_ref())?;
        let pipe = Self::create_named_pipe_file(pipe_path.as_ref())?;
        Ok(Self {
            pipe,
            pipe_path: pipe_path.as_ref().to_path_buf(),
            marker: PhantomData,
        })
    }

    pub fn write_message(&mut self, data: &T) -> std::io::Result<()> {
        let message = WriterTransferMessage::Data(data);
        let serialized = bincode::serialize(&message)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        self.write_all(&serialized)?;
        self.flush()
    }

    pub fn write_done_message(&mut self) -> std::io::Result<()> {
        let message: WriterTransferMessage<T> = WriterTransferMessage::Done;
        let serialized = bincode::serialize(&message)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        self.write_all(&serialized)?;
        self.flush()
    }

    fn create_named_pipe_file(pipe_path: &Path) -> std::io::Result<File> {
        if pipe_path.exists() {
            std::fs::remove_file(pipe_path)?;
        }

        fifo_file::create_fifo(pipe_path, 0o600)?;

        let permissions = std::fs::Permissions::from_mode(0o666);
        std::fs::set_permissions(pipe_path, permissions)?;

        // This is expected to block until a consumer connects.
        File::create(pipe_path)
    }

    fn delete_named_pipe_file(pipe_path: &Path) -> std::io::Result<()> {
        if pipe_path.exists() {
            std::fs::remove_file(pipe_path)?;
        }

        Ok(())
    }
}

impl<T: Serialize> Write for WriterTransferProducer<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.pipe.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.pipe.flush()
    }
}

impl<T: Serialize> Drop for WriterTransferProducer<T> {
    fn drop(&mut self) {
        let pipe_path = self.pipe_path.clone();

        // panicking during drop() is considered bad and with pgrx
        // it's particularly bad as it can raise Postgres ERRORs
        // after we've already found ourselves in a transaction ABORT state
        //
        // If things go wrong, the best we can do is WARN the user about it
        if let Err(err) = self.write_done_message() {
            warn!("error sending writer transfer done message: {err:?}")
        };
        if let Err(err) = std::fs::remove_file(&pipe_path) {
            warn!("error removing named pipe path {pipe_path:?}: {err:?}");
        }
    }
}

pub fn read_stream<'a, T, P>(
    pipe_path: P,
) -> Result<WriterTransferMessageIterator<BufReader<File>, T>, ServerError>
where
    P: AsRef<Path>,
    T: DeserializeOwned + 'a,
{
    let pipe_path_ref = pipe_path.as_ref();

    // Wait for the client to create the pipe.
    while !pipe_path_ref.exists() {
        thread::sleep(Duration::from_millis(10));
    }

    let pipe_file = std::fs::OpenOptions::new()
        .read(true)
        .open(pipe_path_ref)
        .map_err(ServerError::OpenPipeFile)?;

    let reader = BufReader::new(pipe_file);
    Ok(WriterTransferMessageIterator::new(reader))
}

#[cfg(test)]
mod tests {
    use crate::{
        fixtures::*,
        writer::{transfer, SearchDocument, WriterRequest, WriterTransferPipeFilePath},
    };
    use pretty_assertions::assert_eq;
    use rstest::*;
    use std::{path::Path, thread};

    #[rstest]
    fn test_producer_consumer_read_write(
        mock_dir: MockWriterDirectory,
        simple_doc: SearchDocument,
    ) {
        let WriterTransferPipeFilePath(pipe_path) =
            mock_dir.writer_transfer_pipe_path(true).unwrap();

        let writer_request = WriterRequest::Insert {
            directory: mock_dir.writer_dir,
            document: simple_doc,
        };

        // The producer will block until we read from with with read_stream, so we run it
        // in another thread.
        let writer_request_clone = writer_request.clone();
        let pipe_path_clone = pipe_path.clone();
        thread::spawn(move || {
            let mut producer =
                super::WriterTransferProducer::<WriterRequest>::new(&pipe_path_clone).unwrap();
            producer.write_message(&writer_request_clone).unwrap();
            producer.write_message(&writer_request_clone).unwrap();
            producer.write_message(&writer_request_clone).unwrap();
            producer.write_message(&writer_request_clone).unwrap();
        });

        for incoming in transfer::read_stream::<WriterRequest, &Path>(&pipe_path).unwrap() {
            assert_eq!(incoming.unwrap(), writer_request)
        }
    }
}
