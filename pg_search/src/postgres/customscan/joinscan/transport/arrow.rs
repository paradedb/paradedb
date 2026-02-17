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
}

impl DsmStream {
    async fn new(
        multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
        stream_id: PhysicalStreamId,
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
                self.multiplexer
                    .lock()
                    .poll_read_for_stream(self.stream_id, cx)
            })
            .await;

            match chunk {
                Ok(Some(vec)) => {
                    self.accumulated.extend_from_slice(&vec);
                    // Loop to try decoding again
                }
                Ok(None) => {
                    // EOS
                    if let Err(e) = self.decoder.finish() {
                        if self.accumulated.is_empty() {
                            pgrx::warning!(
                                "StreamDecoder finish error for S{} (ignored as buffer empty): {}",
                                self.stream_id.0,
                                e
                            );
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

pub fn dsm_reader(
    multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
    logical_stream_id: LogicalStreamId,
    sender_id: ParticipantId,
    schema: SchemaRef,
) -> SendableRecordBatchStream {
    let physical_stream_id = PhysicalStreamId::new(logical_stream_id, sender_id);

    let stream = try_stream! {
        let mut dsm_stream = DsmStream::new(multiplexer, physical_stream_id).await?;

        while let Some(batch) = dsm_stream.next_batch().await? {
            yield batch;
        }
    };

    Box::pin(RecordBatchStreamAdapter::new(schema, Box::pin(stream)))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::customscan::joinscan::transport::shmem::test_utils::TestBuffer;
    use crate::postgres::customscan::joinscan::transport::shmem::SignalBridge;
    use crate::postgres::customscan::joinscan::transport::TransportMesh;
    use arrow_array::{Int32Array, RecordBatch};
    use arrow_schema::{DataType, Field, Schema};
    use futures::StreamExt;
    use std::sync::Arc;

    #[pgrx::pg_test]
    fn test_dsm_multiplexed_basic() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
            let batch1 = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
            )
            .unwrap();
            let batch2 = RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(Int32Array::from(vec![4, 5, 6]))],
            )
            .unwrap();

            let buffer_size = 1024 * 1024;
            let buf = TestBuffer::new(buffer_size);

            // Setup Bridge
            let bridge = SignalBridge::new(ParticipantId(0), uuid::Uuid::new_v4())
                .await
                .unwrap();
            let bridge = Arc::new(bridge);

            let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let transport = TransportMesh {
                mux_writers: vec![writer_mux.clone()],
                mux_readers: vec![reader_mux.clone()],
                bridge,
            };
            let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
                transport,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

            let mut writer1 = DsmWriter::new(
                writer_mux.clone(),
                LogicalStreamId(1),
                ParticipantId(0),
                schema.clone(),
            );
            let mut writer2 = DsmWriter::new(
                writer_mux.clone(),
                LogicalStreamId(2),
                ParticipantId(0),
                schema.clone(),
            );

            let reader1 = dsm_reader(
                reader_mux.clone(),
                LogicalStreamId(1),
                ParticipantId(0),
                schema.clone(),
            );
            let reader2 = dsm_reader(
                reader_mux.clone(),
                LogicalStreamId(2),
                ParticipantId(0),
                schema.clone(),
            );

            // Write synchronously (in this thread context, effectively)
            writer1.write_batch(&batch1).unwrap();
            writer2.write_batch(&batch2).unwrap();
            writer1.finish().unwrap();
            writer2.finish().unwrap();
            writer_mux.lock().finish().unwrap();

            // Read
            let b1 = reader1.collect::<Vec<_>>().await;
            let b2 = reader2.collect::<Vec<_>>().await;

            assert_eq!(b1.len(), 1);
            assert_eq!(b1[0].as_ref().unwrap().num_rows(), 3);
            assert_eq!(b2.len(), 1);
            assert_eq!(b2[0].as_ref().unwrap().num_rows(), 3);
        });
    }

    #[pgrx::pg_test]
    fn test_dsm_multiplexed_high_volume() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
            let num_batches = 100;
            let num_streams = 5;

            let buffer_size = 10 * 1024 * 1024;
            let buf = TestBuffer::new(buffer_size);

            // Setup Bridge
            let bridge = SignalBridge::new(ParticipantId(0), uuid::Uuid::new_v4())
                .await
                .unwrap();
            let bridge = Arc::new(bridge);

            let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let transport = TransportMesh {
                mux_writers: vec![writer_mux.clone()],
                mux_readers: vec![reader_mux.clone()],
                bridge,
            };
            let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
                transport,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

            let mut writers = Vec::new();
            let mut readers = Vec::new();

            for i in 0..num_streams {
                writers.push(DsmWriter::new(
                    writer_mux.clone(),
                    LogicalStreamId(i as u16),
                    ParticipantId(0),
                    schema.clone(),
                ));
                readers.push(dsm_reader(
                    reader_mux.clone(),
                    LogicalStreamId(i as u16),
                    ParticipantId(0),
                    schema.clone(),
                ));
            }

            // 1. Write everything synchronously first to avoid hang in block_on without cross-process waker
            for b in 0..num_batches {
                for writer in writers.iter_mut() {
                    let batch = RecordBatch::try_new(
                        schema.clone(),
                        vec![Arc::new(Int32Array::from(vec![b as i32]))],
                    )
                    .unwrap();
                    writer.write_batch(&batch).unwrap();
                }
            }
            for writer in writers {
                writer.finish().unwrap();
            }
            writer_mux.lock().finish().unwrap();

            let mut readers = readers;
            for _i in 0..num_streams {
                let reader = readers.remove(0);
                let batches = reader.collect::<Vec<_>>().await;
                assert_eq!(batches.len(), num_batches);
                for (b, batch) in batches.iter().enumerate() {
                    let batch = batch.as_ref().unwrap();
                    assert_eq!(
                        batch
                            .column(0)
                            .as_any()
                            .downcast_ref::<Int32Array>()
                            .unwrap()
                            .value(0),
                        b as i32
                    );
                }
            }
        });
    }

    #[pgrx::pg_test]
    fn test_dsm_empty_stream() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
            let buffer_size = 1024 * 1024;
            let buf = TestBuffer::new(buffer_size);

            let bridge = SignalBridge::new_dummy();

            let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                buf.base_ptr,
                buf.capacity,
                bridge.clone(),
                ParticipantId(0),
            )));
            let transport = TransportMesh {
                mux_writers: vec![writer_mux.clone()],
                mux_readers: vec![reader_mux.clone()],
                bridge,
            };
            let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
                transport,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

            let writer = DsmWriter::new(
                writer_mux.clone(),
                LogicalStreamId(1),
                ParticipantId(0),
                schema.clone(),
            );
            let reader = dsm_reader(
                reader_mux.clone(),
                LogicalStreamId(1),
                ParticipantId(0),
                schema.clone(),
            );

            // Write NOTHING, just finish
            writer.finish().unwrap();
            writer_mux.lock().finish().unwrap();

            let batches = reader.collect::<Vec<_>>().await;
            assert_eq!(batches.len(), 0);
        });
    }

    #[pgrx::pg_test]
    fn test_dsm_batch_too_large() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![0; 1024]))],
        )
        .unwrap();

        let buffer_size = 1024;
        let buf = TestBuffer::new(buffer_size);

        let bridge = SignalBridge::new_dummy();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            buf.base_ptr,
            buf.capacity,
            bridge,
            ParticipantId(0),
        )));
        let mut writer = DsmWriter::new(
            writer_mux.clone(),
            LogicalStreamId(1),
            ParticipantId(0),
            schema.clone(),
        );

        let result = writer.write_batch(&batch);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds ring buffer capacity"));
    }
}
