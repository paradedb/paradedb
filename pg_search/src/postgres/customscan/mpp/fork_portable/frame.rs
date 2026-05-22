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

//! Multi-channel Arrow IPC frame codec.
//!
//! Every wire message carries a fixed 16-byte [`MppFrameHeader`] prefix
//! `(magic, flags, stage_id, partition)` so a single underlying byte queue can
//! interleave frames for many logical `(stage_id, partition)` channels.
//! Receivers demultiplex by header before touching Arrow IPC.
//!
//! Wire format:
//!
//! ```text
//! [ magic | flags | stage_id | partition ] [ Arrow IPC stream bytes ]
//! |---------- 16 bytes --------|           |---- variable ----|
//! ```
//!
//! No `pgrx::*` references — see this submodule's `mod.rs` for the
//! fork-portability story.

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::DataFusionError;

/// Magic bytes "MPPF" (MPP Frame) at the start of every wire message.
/// Lets receivers reject misrouted / corrupt frames before they hit Arrow IPC.
const MPP_FRAME_MAGIC: u32 = 0x4D505046;

/// Wire-format size of [`MppFrameHeader`] in bytes. Asserted at compile time
/// below via `const _: ()`.
pub(in crate::postgres::customscan::mpp) const MPP_FRAME_HEADER_SIZE: usize = 16;

/// Kind of payload following [`MppFrameHeader`].
///
/// `Batch` is the common case. The header is followed by an Arrow IPC stream containing one
/// `RecordBatch`. `Eof` carries no payload. It signals the receiver that the named
/// `(stage_id, partition)` channel is done, even though the underlying byte queue may still
/// carry frames for other channels.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::postgres::customscan::mpp) enum MppFrameKind {
    Batch = 0,
    Eof = 1,
}

/// 16-byte prefix on every transport frame.
///
/// The fixed layout `[magic, flags, stage_id, partition]` (4×u32) is what
/// senders prepend before the Arrow IPC stream bytes and what receivers
/// parse before deciding which channel buffer the payload belongs to.
///
/// The `flags` word currently encodes [`MppFrameKind`] in its low byte (mask
/// `0x0000_00FF`); the upper 24 bits are reserved-must-be-zero and are
/// validated at parse time so a future use can repurpose them without a
/// wire-format break.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MppFrameHeader {
    pub magic: u32,
    pub flags: u32,
    pub stage_id: u32,
    pub partition: u32,
}

/// Bit mask in [`MppFrameHeader::flags`] for the [`MppFrameKind`] discriminant.
const FRAME_KIND_MASK: u32 = 0x0000_00FF;

const _: () = {
    // shm_mq slot layout calculations depend on this being exact.
    assert!(std::mem::size_of::<MppFrameHeader>() == MPP_FRAME_HEADER_SIZE);
};

impl MppFrameHeader {
    /// Build a `Batch` header for the given `(stage_id, partition)`.
    pub fn batch(stage_id: u32, partition: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: MppFrameKind::Batch as u32,
            stage_id,
            partition,
        }
    }

    /// Build an `Eof` header for the given `(stage_id, partition)`. Carries no payload; receivers
    /// route it to the channel buffer's source-done counter. Emitted after a producer fragment's
    /// per-partition stream exhausts (or errors), and consumed by the matching channel buffer's
    /// `notify_source_done`.
    pub fn eof(stage_id: u32, partition: u32) -> Self {
        Self {
            magic: MPP_FRAME_MAGIC,
            flags: MppFrameKind::Eof as u32,
            stage_id,
            partition,
        }
    }

    /// Read the kind out of `flags`. Returns an error if the kind byte is
    /// unknown or if any reserved upper bit is set, which catches wire-format
    /// drift early.
    pub(in crate::postgres::customscan::mpp) fn kind(
        &self,
    ) -> Result<MppFrameKind, DataFusionError> {
        let reserved = self.flags & !FRAME_KIND_MASK;
        if reserved != 0 {
            return Err(DataFusionError::Internal(format!(
                "mpp: reserved frame flag bits set ({reserved:#x})"
            )));
        }
        match self.flags & FRAME_KIND_MASK {
            0 => Ok(MppFrameKind::Batch),
            1 => Ok(MppFrameKind::Eof),
            other => Err(DataFusionError::Internal(format!(
                "mpp: unknown frame kind {other:#x}"
            ))),
        }
    }

    /// Serialize into the first `MPP_FRAME_HEADER_SIZE` bytes of `out`.
    /// `out.len()` must be `>= MPP_FRAME_HEADER_SIZE`.
    fn write_to(&self, out: &mut [u8]) {
        debug_assert!(out.len() >= MPP_FRAME_HEADER_SIZE);
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.stage_id.to_le_bytes());
        out[12..16].copy_from_slice(&self.partition.to_le_bytes());
    }

    /// Parse from the first `MPP_FRAME_HEADER_SIZE` bytes of `bytes`. Returns
    /// `Err` if the slice is too short or the magic doesn't match.
    fn parse(bytes: &[u8]) -> Result<Self, DataFusionError> {
        if bytes.len() < MPP_FRAME_HEADER_SIZE {
            // No encoder in this file emits sub-header output, so a short frame means the
            // byte queue stitched together payloads from different senders. Hex-dump the bytes
            // so the source is identifiable from log output without a debugger.
            let hex = bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            return Err(DataFusionError::Internal(format!(
                "mpp: frame too short for header ({} < {}); bytes = [{hex}]",
                bytes.len(),
                MPP_FRAME_HEADER_SIZE
            )));
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != MPP_FRAME_MAGIC {
            return Err(DataFusionError::Internal(format!(
                "mpp: bad frame magic {magic:#x} (expected {MPP_FRAME_MAGIC:#x})"
            )));
        }
        Ok(Self {
            magic,
            flags: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            stage_id: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            partition: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        })
    }
}

/// Serialize `batch` into `buf` with a 16-byte [`MppFrameHeader`] prefix
/// addressing it to `(stage_id, partition)`. Wire format:
///
/// ```text
/// [ magic | flags | stage_id | partition ] [ Arrow IPC stream bytes ]
/// |---------- 16 bytes --------|           |---- variable ----|
/// ```
///
/// Caller is expected to hold `buf` alive across many encodes so the peak-sized
/// allocation amortizes (~500 KB/batch on the 25M GROUP BY bench).
pub(in crate::postgres::customscan::mpp) fn encode_frame_into(
    header: MppFrameHeader,
    batch: &RecordBatch,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    header.write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    let mut writer = StreamWriter::try_new(&mut *buf, batch.schema_ref())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(())
}

/// Serialize a payload-less [`MppFrameKind::Eof`] frame for `(stage_id, partition)`
/// into `buf`. Receivers read this as a 16-byte message and route it to the
/// channel buffer's source-done counter without touching Arrow IPC.
///
/// Used when a producer fragment's per-partition stream exhausts, so the
/// receiver's `(stage_id, partition)` channel buffer transitions to `Eof` even
/// though the multiplexed underlying queue stays attached for other channels.
pub(in crate::postgres::customscan::mpp) fn encode_eof_frame_into(
    stage_id: u32,
    partition: u32,
    buf: &mut Vec<u8>,
) -> Result<(), DataFusionError> {
    buf.clear();
    buf.resize(MPP_FRAME_HEADER_SIZE, 0);
    MppFrameHeader::eof(stage_id, partition).write_to(&mut buf[..MPP_FRAME_HEADER_SIZE]);
    Ok(())
}

/// Inverse of [`encode_frame_into`]. Parses the 16-byte header and, for `Batch` frames, decodes
/// the trailing Arrow IPC stream. `Eof` frames return `(header, None)`. Receivers branch on
/// `header.kind()` to decide routing.
pub(in crate::postgres::customscan::mpp) fn decode_frame(
    bytes: &[u8],
) -> Result<(MppFrameHeader, Option<RecordBatch>), DataFusionError> {
    let header = MppFrameHeader::parse(bytes)?;
    match header.kind()? {
        MppFrameKind::Eof => {
            if bytes.len() != MPP_FRAME_HEADER_SIZE {
                return Err(DataFusionError::Internal(format!(
                    "mpp: Eof frame carries payload ({} > {})",
                    bytes.len(),
                    MPP_FRAME_HEADER_SIZE
                )));
            }
            Ok((header, None))
        }
        MppFrameKind::Batch => {
            let payload = &bytes[MPP_FRAME_HEADER_SIZE..];
            let mut reader = StreamReader::try_new(payload, None)?;
            let batch = reader.next().ok_or_else(|| {
                DataFusionError::Execution("mpp: empty arrow-ipc stream in decode_frame".into())
            })??;
            Ok((header, Some(batch)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(names)]).unwrap()
    }

    #[test]
    fn frame_round_trips_a_batch_with_header() {
        let orig = sample_batch(64);
        let header = MppFrameHeader::batch(7, 3);
        let mut buf = Vec::with_capacity(1024);
        encode_frame_into(header, &orig, &mut buf).expect("encode_frame");

        let (parsed, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(parsed, header);
        assert_eq!(parsed.kind().unwrap(), MppFrameKind::Batch);
        let decoded = batch_opt.expect("Batch frame must carry a payload");
        assert_eq!(decoded.num_rows(), 64);
        assert_eq!(decoded.schema(), orig.schema());
        assert_eq!(decoded.num_columns(), orig.num_columns());
        for col in 0..orig.num_columns() {
            assert_eq!(orig.column(col).as_ref(), decoded.column(col).as_ref());
        }
    }

    #[test]
    fn frame_round_trips_eof() {
        let mut buf = Vec::new();
        encode_eof_frame_into(2, 5, &mut buf).expect("encode_eof");
        assert_eq!(buf.len(), MPP_FRAME_HEADER_SIZE);

        let (header, batch_opt) = decode_frame(&buf).expect("decode_frame");
        assert_eq!(header, MppFrameHeader::eof(2, 5));
        assert_eq!(header.kind().unwrap(), MppFrameKind::Eof);
        assert!(batch_opt.is_none());
    }

    #[test]
    fn frame_rejects_short_message() {
        let too_short = vec![0u8; MPP_FRAME_HEADER_SIZE - 1];
        let err = decode_frame(&too_short).expect_err("short frame must fail");
        assert!(format!("{err}").contains("too short"));
    }

    #[test]
    fn frame_rejects_bad_magic() {
        // Explicit non-zero, non-magic prefix. Don't rely on the
        // happenstance that 0u32 != MPP_FRAME_MAGIC.
        let mut bad = vec![0u8; MPP_FRAME_HEADER_SIZE];
        bad[0..4].copy_from_slice(&0xCAFEBABE_u32.to_le_bytes());
        let err = decode_frame(&bad).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
        bad[0..4].copy_from_slice(&0xDEADBEEF_u32.to_le_bytes());
        let err = decode_frame(&bad).expect_err("bad magic must fail");
        assert!(format!("{err}").contains("bad frame magic"));
    }

    #[test]
    fn frame_rejects_unknown_kind() {
        let header = MppFrameHeader {
            magic: MPP_FRAME_MAGIC,
            flags: 0x42, // unknown kind byte, no reserved bits set
            stage_id: 0,
            partition: 0,
        };
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        let err = decode_frame(&buf).expect_err("unknown kind must fail");
        assert!(format!("{err}").contains("unknown frame kind"));
    }

    #[test]
    fn frame_rejects_reserved_flag_bits() {
        // Any bit above the low byte of `flags` is reserved-must-be-zero;
        // setting one should trip `kind()` before the kind byte is consulted.
        let header = MppFrameHeader {
            magic: MPP_FRAME_MAGIC,
            flags: 0x0000_0100, // bit 8 set, kind byte 0 (would be Batch)
            stage_id: 0,
            partition: 0,
        };
        let mut buf = vec![0u8; MPP_FRAME_HEADER_SIZE];
        header.write_to(&mut buf);
        let err = decode_frame(&buf).expect_err("reserved bit must fail");
        assert!(format!("{err}").contains("reserved frame flag bits"));
    }

    #[test]
    fn frame_eof_with_payload_is_rejected() {
        let mut buf = Vec::with_capacity(32);
        encode_eof_frame_into(0, 0, &mut buf).expect("encode_eof");
        buf.push(0xAB); // smuggle a payload byte after the Eof header
        let err = decode_frame(&buf).expect_err("Eof+payload must fail");
        assert!(format!("{err}").contains("Eof frame carries payload"));
    }

    #[test]
    fn codec_round_trips_many_batch_sizes() {
        let mut buf = Vec::with_capacity(1024);
        for rows in [0, 1, 7, 64, 1024] {
            let orig = sample_batch(rows);
            encode_frame_into(MppFrameHeader::batch(0, 0), &orig, &mut buf).expect("encode");
            let (_header, decoded) = decode_frame(&buf).expect("decode");
            let decoded = decoded.expect("Batch frame must carry a payload");
            assert_eq!(orig.num_rows(), decoded.num_rows());
        }
    }
}
