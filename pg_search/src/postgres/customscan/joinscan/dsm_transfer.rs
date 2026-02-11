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
//! It builds upon the generic DSM stream abstraction provided by `dsm_stream`.

use std::io::Write;
use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_buffer::Buffer;
use arrow_ipc::reader::StreamDecoder;
use arrow_ipc::writer::StreamWriter;
use arrow_schema::SchemaRef;
use async_stream::try_stream;
use datafusion::common::Result;
use datafusion::execution::SendableRecordBatchStream;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use parking_lot::Mutex;

// Re-export commonly used types from dsm_stream for convenience in other modules
pub use crate::postgres::customscan::joinscan::dsm_stream::{
    DsmStreamWriterAdapter, MultiplexedDsmReader, MultiplexedDsmWriter, RingBufferHeader,
    SignalBridge,
};

/// A writer for a single logical stream within a multiplexed DSM region.
pub struct DsmSharedMemoryWriter {
    writer: StreamWriter<DsmStreamWriterAdapter>,
}

unsafe impl Send for DsmSharedMemoryWriter {}

impl DsmSharedMemoryWriter {
    pub fn new(
        multiplexer: Arc<Mutex<MultiplexedDsmWriter>>,
        logical_stream_id: u32,
        sender_index: usize,
        _dest_index: usize,
        schema: SchemaRef,
    ) -> Self {
        // Construct a unique Physical Stream ID for this writer/channel pair.
        // We pack the Logical ID (from the plan) and the Sender Index (who is writing)
        // into a single u32. This assumes < 64k streams and < 64k workers, which is safe for Postgres.
        let physical_stream_id = (logical_stream_id << 16) | ((sender_index as u32) & 0xFFFF);

        let adapter = DsmStreamWriterAdapter::new(multiplexer, physical_stream_id);
        // This will immediately write the Arrow schema to the ring buffer via the multiplexer.
        let mut writer = StreamWriter::try_new(adapter, &schema)
            .expect("Failed to create Arrow IPC StreamWriter");
        // Flush to ensure schema is sent immediately.
        writer.get_mut().flush().unwrap();

        Self { writer }
    }

    /// Writes a `RecordBatch` to the multiplexed stream.
    ///
    /// This method serializes the batch into the logical stream's internal buffer,
    /// which is then flushed as a framed message into the physical DSM ring buffer.
    /// If the physical buffer is full, it returns `ErrorKind::WouldBlock`.
    pub fn write_batch(&mut self, batch: &RecordBatch) -> Result<()> {
        if self.writer.get_mut().is_empty() {
            self.writer.write(batch).map_err(|e| match e {
                arrow_schema::ArrowError::IoError(_, io_err) => {
                    datafusion::common::DataFusionError::IoError(io_err)
                }
                _ => datafusion::common::DataFusionError::Internal(format!(
                    "Failed to write batch to IPC: {e}"
                )),
            })?;
        }

        // Flush after each batch to ensure it's available for reading.
        self.writer
            .get_mut()
            .flush()
            .map_err(datafusion::common::DataFusionError::IoError)?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.writer.finish().map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!(
                "Failed to finish IPC stream: {e}"
            ))
        })?;
        self.writer.get_mut().flush().map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!("Failed to flush IPC: {e}"))
        })?;

        // Send EOS signal (len=0 frame)
        let inner = self.writer.get_mut();
        // We need to lock the multiplexer to close the stream.
        // Note: inner.multiplexer is private in dsm_stream.rs but the struct DsmStreamWriterAdapter is in dsm_stream.rs
        // We are in dsm_transfer.rs.
        // We need to expose the multiplexer or add a close method to DsmStreamWriterAdapter.
        // Let's modify DsmStreamWriterAdapter in dsm_stream.rs to have a close() method first.
        // For now, assuming I will add it, let's write the code here to call it.
        inner.close_stream().map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!("Failed to close stream: {e}"))
        })?;

        Ok(())
    }
}

struct DsmStream {
    multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
    stream_id: u32,
    finished: bool,
    decoder: StreamDecoder,
    accumulated: Vec<u8>,
}

impl DsmStream {
    async fn new(multiplexer: Arc<Mutex<MultiplexedDsmReader>>, stream_id: u32) -> Result<Self> {
        // Send StartStream signal to the writer
        multiplexer.lock().start_stream(stream_id).map_err(|e| {
            datafusion::common::DataFusionError::Internal(format!(
                "Failed to send StartStream signal: {e}"
            ))
        })?;

        Ok(Self {
            multiplexer,
            stream_id,
            finished: false,
            decoder: StreamDecoder::new(),
            accumulated: Vec::new(),
        })
    }

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
                        // Need more data, buffer remains in accumulated (or drained if fully consumed?)
                        // decode() advances buffer. If it returns None, it means "need more data".
                        // BUT if we consumed *some* bytes but not enough for a full batch?
                        // decode() updates `buffer`. We must update `accumulated`.
                        let consumed = self.accumulated.len() - buffer.len();
                        self.accumulated.drain(0..consumed);
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
                    self.decoder.finish().map_err(|e| {
                        datafusion::common::DataFusionError::Internal(format!(
                            "StreamDecoder finish error: {e}"
                        ))
                    })?;
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

impl Drop for DsmStream {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if std::thread::panicking() {
            return;
        }

        let _ = self.multiplexer.lock().cancel_stream(self.stream_id);
    }
}

pub fn dsm_shared_memory_reader(
    multiplexer: Arc<Mutex<MultiplexedDsmReader>>,
    logical_stream_id: u32,
    sender_index: usize,
    schema: SchemaRef,
) -> SendableRecordBatchStream {
    let physical_stream_id = (logical_stream_id << 16) | ((sender_index as u32) & 0xFFFF);

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
    use crate::postgres::customscan::joinscan::dsm_stream::RingBufferHeader;
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
            let mut storage = vec![0u8; size_of::<RingBufferHeader>() + buffer_size];
            let header = storage.as_mut_ptr() as *mut RingBufferHeader;
            let data = unsafe { storage.as_mut_ptr().add(size_of::<RingBufferHeader>()) };

            unsafe {
                RingBufferHeader::init(header, 0);
            }

            // Setup Bridge
            let bridge = SignalBridge::new(0, uuid::Uuid::new_v4()).await.unwrap();
            let bridge = Arc::new(bridge);

            let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                buffer_size,
                bridge.clone(),
                0,
            )));
            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                buffer_size,
                bridge.clone(),
                0,
            )));
            let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
                total_participants: 1,
                mux_writers: vec![writer_mux.clone()],
                mux_readers: vec![reader_mux.clone()],
                bridge,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

            let mut writer1 =
                DsmSharedMemoryWriter::new(writer_mux.clone(), 1, 0, 0, schema.clone());
            let mut writer2 =
                DsmSharedMemoryWriter::new(writer_mux.clone(), 2, 0, 0, schema.clone());

            let reader1 = dsm_shared_memory_reader(reader_mux.clone(), 1, 0, schema.clone());
            let reader2 = dsm_shared_memory_reader(reader_mux.clone(), 2, 0, schema.clone());

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
            let mut storage = vec![0u8; size_of::<RingBufferHeader>() + buffer_size];
            let header = storage.as_mut_ptr() as *mut RingBufferHeader;
            let data = unsafe { storage.as_mut_ptr().add(size_of::<RingBufferHeader>()) };

            unsafe {
                RingBufferHeader::init(header, 0);
            }

            // Setup Bridge
            let bridge = SignalBridge::new(0, uuid::Uuid::new_v4()).await.unwrap();
            let bridge = Arc::new(bridge);

            let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
                header,
                data,
                buffer_size,
                bridge.clone(),
                0,
            )));
            let reader_mux = Arc::new(Mutex::new(MultiplexedDsmReader::new(
                header,
                data,
                buffer_size,
                bridge.clone(),
                0,
            )));
            let mesh = crate::postgres::customscan::joinscan::exchange::DsmMesh {
                total_participants: 1,
                mux_writers: vec![writer_mux.clone()],
                mux_readers: vec![reader_mux.clone()],
                bridge,
                registry: Mutex::new(
                    crate::postgres::customscan::joinscan::exchange::StreamRegistry::default(),
                ),
            };
            crate::postgres::customscan::joinscan::exchange::register_dsm_mesh(mesh);

            let mut writers = Vec::new();
            let mut readers = Vec::new();

            for i in 0..num_streams {
                writers.push(DsmSharedMemoryWriter::new(
                    writer_mux.clone(),
                    i as u32,
                    0,
                    0,
                    schema.clone(),
                ));
                readers.push(dsm_shared_memory_reader(
                    reader_mux.clone(),
                    i as u32,
                    0,
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
    fn test_dsm_batch_too_large() {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(Int32Array::from(vec![0; 1024]))],
        )
        .unwrap();

        let buffer_size = 1024;
        let mut storage = vec![0u8; size_of::<RingBufferHeader>() + buffer_size];
        let header = storage.as_mut_ptr() as *mut RingBufferHeader;
        let data = unsafe { storage.as_mut_ptr().add(size_of::<RingBufferHeader>()) };

        unsafe {
            RingBufferHeader::init(header, 0);
        }

        let bridge = SignalBridge::new_dummy();
        let writer_mux = Arc::new(Mutex::new(MultiplexedDsmWriter::new(
            header,
            data,
            buffer_size,
            bridge,
            0,
        )));
        let mut writer = DsmSharedMemoryWriter::new(writer_mux.clone(), 1, 0, 0, schema.clone());

        let result = writer.write_batch(&batch);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds ring buffer capacity"));
    }
}
