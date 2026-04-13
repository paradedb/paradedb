// Copyright (c) 2023-2026 ParadeDB, Inc.
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

//! # Shared Memory Transfer & Signaling
//!
//! This module implements the infrastructure for streaming Arrow `RecordBatch`es
//! between processes via Shared Memory (DSM) ring buffers.
//!
//! It builds upon the generic DSM stream abstraction provided by `shmem`.

use arrow_array::RecordBatch;
use arrow_buffer::Buffer;
use arrow_ipc::reader::StreamDecoder;
use arrow_ipc::writer::{
    CompressionContext, DictionaryTracker, IpcDataGenerator, IpcWriteOptions, StreamWriter,
};
use arrow_schema::SchemaRef;
use async_stream::try_stream;
use datafusion::common::Result;
use datafusion::execution::SendableRecordBatchStream;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

// Use types from shmem
use super::shmem::{
    LogicalStreamId, MultiplexedDsmReader, MultiplexedDsmWriter, ParticipantId, PhysicalStreamId,
};

/// A writer for a single logical stream within a multiplexed DSM region.
///
/// This writer handles the serialization of Arrow `RecordBatch`es into the Shared Memory Ring Buffer.
/// It uses a "1-copy" strategy where batches are serialized into a local `Vec<u8>` (Copy 1)
/// and then written directly to the Ring Buffer (Copy 2) when space permits.
pub struct DsmWriter {
    multiplexer: Arc<Mutex<MultiplexedDsmWriter>>,
    stream_id: PhysicalStreamId,
    generator: IpcDataGenerator,
    tracker: DictionaryTracker,
    options: IpcWriteOptions,
    compression_context: CompressionContext,
    pending_messages: VecDeque<Vec<u8>>,
}

unsafe impl Send for DsmWriter {}

impl DsmWriter {
    pub fn new(
        multiplexer: Arc<Mutex<MultiplexedDsmWriter>>,
        logical_stream_id: LogicalStreamId,
        sender_id: ParticipantId,
        schema: SchemaRef,
    ) -> Self {
        // Construct a unique Physical Stream ID for this writer/channel pair.
        let physical_stream_id = PhysicalStreamId::new(logical_stream_id, sender_id);
        let options = IpcWriteOptions::default();
        let generator = IpcDataGenerator::default();
        let tracker = DictionaryTracker::new(false);
        let compression_context = CompressionContext::default();
        let mut pending_messages = VecDeque::new();

        // Generate Schema message
        // We use a temporary writer to generate the schema message correctly wrapped
        let mut schema_buffer = Vec::new();
        let _writer = StreamWriter::try_new(&mut schema_buffer, &schema)
            .expect("Failed to create Arrow IPC StreamWriter for schema generation");

        pending_messages.push_back(schema_buffer);

        Self {
            multiplexer,
            stream_id: physical_stream_id,
            generator,
            tracker,
            options,
            compression_context,
            pending_messages,
        }
    }

    /// Writes a `RecordBatch` to the multiplexed stream.
    pub fn write_batch(&mut self, batch: &RecordBatch) -> Result<()> {
        // 1. Flush any pending messages from previous attempts
        if let Err(e) = self.flush_pending() {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                return Err(datafusion::common::DataFusionError::IoError(e));
            }
            return Err(datafusion::common::DataFusionError::IoError(e));
        }

        // 2. Encode the new batch
        let (dictionaries, encoded_batch) = self
            .generator
            .encode(
                batch,
                &mut self.tracker,
                &self.options,
                &mut self.compression_context,
            )
            .map_err(|e| {
                datafusion::common::DataFusionError::Internal(format!(
                    "Failed to encode batch: {e}"
                ))
            })?;

        // 3. Queue dictionary batches
        for dict_batch in dictionaries {
            let msg = Self::serialize_message(dict_batch, &self.options);
            self.pending_messages.push_back(msg);
        }

        // 4. Queue the record batch
        let msg = Self::serialize_message(encoded_batch, &self.options);
        self.pending_messages.push_back(msg);

        // 5. Try to flush immediately
        self.flush_pending()
            .map_err(datafusion::common::DataFusionError::IoError)
    }

    pub fn finish(mut self) -> Result<()> {
        // Queue Arrow IPC Stream EOS footer
        const EOS_FOOTER: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0];
        self.pending_messages.push_back(EOS_FOOTER.to_vec());

        // Ensure everything is flushed
        loop {
            match self.flush_pending() {
                Ok(_) => {
                    // If queue is empty, we are done
                    if self.pending_messages.is_empty() {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // In a sync finish, we must spin/wait?
                    // But DsmWriter is usually driven by an async loop or sync test.
                    // Ideally we return error and let caller retry, but `finish` signature consumes self.
                    // For now, let's assume if we are calling finish, we should be able to block or fail.
                    // Given the existing code didn't handle WouldBlock on finish well (except via flush error),
                    // we will return the error.
                    return Err(datafusion::common::DataFusionError::IoError(e));
                }
                Err(e) => return Err(datafusion::common::DataFusionError::IoError(e)),
            }
        }

        // Send EOS signal
        self.multiplexer
            .lock()
            .close_stream(self.stream_id)
            .map_err(|e| {
                datafusion::common::DataFusionError::Internal(format!(
                    "Failed to close stream: {e}"
                ))
            })?;

        Ok(())
    }

    fn flush_pending(&mut self) -> std::io::Result<()> {
        let mut mux = self.multiplexer.lock();
        while let Some(msg) = self.pending_messages.front() {
            match mux.write_message(self.stream_id, msg) {
                Ok(_) => {
                    self.pending_messages.pop_front();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn serialize_message(
        encoded: arrow_ipc::writer::EncodedData,
        options: &IpcWriteOptions,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        // This helper from arrow_ipc writes: [continuation:4][len:4][flatbuffer][body]
        arrow_ipc::writer::write_message(&mut buf, encoded, options).unwrap();
        buf
    }
}

struct DsmStream {
    multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
    stream_id: PhysicalStreamId,
    finished: bool,
    decoder: StreamDecoder,
    accumulated: Vec<u8>,
    sanitized: bool,
}

impl DsmStream {
    async fn new(
        multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
        stream_id: PhysicalStreamId,
        sanitized: bool,
    ) -> Result<Self> {
        {
            let mut mux = multiplexer.lock();
            // Send StartStream signal to the writer
            mux.start_stream(stream_id).map_err(|e| {
                datafusion::common::DataFusionError::Internal(format!(
                    "Failed to send StartStream signal: {e}"
                ))
            })?;
        }

        Ok(Self {
            multiplexer,
            stream_id,
            finished: false,
            decoder: StreamDecoder::new(),
            accumulated: Vec::new(),
            sanitized,
        })
    }

    /// Reads and decodes the next `RecordBatch` from the stream.
    ///
    /// This method handles:
    /// 1. Reading chunks from the DSM Ring Buffer.
    /// 2. Reassembling chunks in `self.accumulated`.
    /// 3. Decoding IPC messages (Schema, RecordBatch) from the accumulator.
    ///
    /// # Partial Decodes
    ///
    /// If the `StreamDecoder` consumes bytes but returns `None` (NeedMoreData), it means it successfully
    /// decoded a message (e.g., Schema) but needs more data for the *next* message (e.g., RecordBatch).
    /// In this case, we **must loop again** to try decoding the next message from the *remaining*
    /// bytes in the accumulator before polling the network, as multiple messages might have been
    /// delivered in a single chunk.
    async fn next_batch(&mut self) -> Result<Option<RecordBatch>> {
        loop {
            // 1. Try to decode from accumulated buffer
            if !self.accumulated.is_empty() {
                let mut buffer = Buffer::from(self.accumulated.clone());
                match self.decoder.decode(&mut buffer) {
                    Ok(Some(batch)) => {
                        let consumed = self.accumulated.len() - buffer.len();
                        self.accumulated.drain(0..consumed);
                        return Ok(Some(batch));
                    }
                    Ok(None) => {
                        // Need more data
                        let consumed = self.accumulated.len() - buffer.len();
                        if consumed > 0 {
                            self.accumulated.drain(0..consumed);
                            // We made progress (e.g. decoded Schema). There might be more data in accumulated.
                            continue;
                        }
                    }
                    Err(e) => {
                        return Err(datafusion::common::DataFusionError::Internal(format!(
                            "StreamDecoder error: {e}"
                        )));
                    }
                }
            }

            // 2. Read from DSM
            let chunk = futures::future::poll_fn(|cx| {
                if self.sanitized {
                    self.multiplexer
                        .lock()
                        .poll_read_for_stream_copying(self.stream_id, cx)
                } else {
                    self.multiplexer
                        .lock()
                        .poll_read_for_stream(self.stream_id, cx)
                }
            })
            .await;

            match chunk {
                Ok(Some(mut buffer)) => {
                    // Optimized Path: If accumulated is empty, decode directly from the zero-copy buffer
                    if self.accumulated.is_empty() {
                        match self.decoder.decode(&mut buffer) {
                            Ok(Some(batch)) => {
                                // If there are remaining bytes (e.g. next message partial), stash them
                                if !buffer.is_empty() {
                                    self.accumulated.extend_from_slice(buffer.as_slice());
                                }
                                return Ok(Some(batch));
                            }
                            Ok(None) => {
                                // Consumed some or none?
                                // If consumed everything (unlikely for None), buffer is empty.
                                // If partial, stash remaining.
                                // If we consumed some bytes (e.g. Schema), we might need to loop again?
                                // Decoder updates `buffer` slice to point to remaining data.
                                // We stash remaining data.
                                self.accumulated.extend_from_slice(buffer.as_slice());
                                continue;
                            }
                            Err(e) => {
                                return Err(datafusion::common::DataFusionError::Internal(
                                    format!("StreamDecoder error: {e}"),
                                ));
                            }
                        }
                    } else {
                        // Slow Path: Append to existing accumulator
                        self.accumulated.extend_from_slice(buffer.as_slice());
                    }
                }
                Ok(None) => {
                    // EOS
                    if let Err(e) = self.decoder.finish() {
                        if self.accumulated.is_empty() {
                            // Ignored as harmless if buffer empty
                        } else {
                            return Err(datafusion::common::DataFusionError::Internal(format!(
                                "StreamDecoder finish error for S{}: {}",
                                self.stream_id.0, e
                            )));
                        }
                    }
                    self.finished = true;
                    return Ok(None);
                }
                Err(e) => {
                    return Err(datafusion::common::DataFusionError::Internal(format!(
                        "Failed to read from DSM: {e}"
                    )));
                }
            }
        }
    }
}

crate::impl_safe_drop!(DsmStream, |self| {
    if self.finished {
        return;
    }
    let _ = self.multiplexer.lock().cancel_stream(self.stream_id);
});

/// Creates a record batch stream that reads from a DSM ring buffer.
///
/// # Arguments
///
/// * `multiplexer` - The reader multiplexer.
/// * `logical_stream_id` - The logical stream ID to read.
/// * `sender_id` - The participant ID of the sender.
/// * `schema` - The schema of the stream.
/// * `sanitized` - If true, the stream will perform a deep copy of the data immediately upon reading.
///   This is necessary to prevent deadlocks when feeding blocking operators (like Sort) that might
///   hold references to the zero-copy buffer.
pub fn dsm_reader(
    multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
    logical_stream_id: LogicalStreamId,
    sender_id: ParticipantId,
    schema: SchemaRef,
    sanitized: bool,
) -> SendableRecordBatchStream {
    let physical_stream_id = PhysicalStreamId::new(logical_stream_id, sender_id);

    let stream = try_stream! {
        let mut dsm_stream = DsmStream::new(multiplexer, physical_stream_id, sanitized).await?;

        while let Some(batch) = dsm_stream.next_batch().await? {
            yield batch;
        }
    };

    Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(stream)))
}

// TODO(#4152): Arrow IPC tests removed temporarily — they depend on exchange::DsmMesh,
// exchange::StreamRegistry, and exchange::register_dsm_mesh which will be added in Phase 2.
// Tests will be restored when exchange.rs is ported.
