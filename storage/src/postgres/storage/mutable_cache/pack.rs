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

//! Packing and unpacking of Tantivy segment component files into a single contiguous buffer.
//!
//! # Packed Segment Binary Format
//!
//! An indexed Tantivy segment comprises multiple component files (`.term`, `.idx`, `.pos`,
//! `.postings`, `.fieldnorm`, `.fast`, `.store`, etc.). To cache a segment in a single contiguous
//! allocation in the shared-memory mutable cache, all files belonging to the segment are packed
//! into a single byte buffer with an 8-byte aligned Table of Contents (TOC):
//!
//! ```text
//! +--------------------------------------------------------------------------------+
//! | Header: magic (u32), version (u32), file_count (u32), total_bytes (u64)       |
//! +--------------------------------------------------------------------------------+
//! | Table of Contents (TOC):                                                       |
//! |   Entry 0: name_len (u16), name [u8; 62], offset (u64), len (u64)              |
//! |   Entry 1: name_len (u16), name [u8; 62], offset (u64), len (u64)              |
//! |   ...                                                                          |
//! +--------------------------------------------------------------------------------+
//! | File Payloads (8-byte aligned offsets):                                        |
//! |   [ File 0 Bytes ] [ File 1 Bytes ] [ File 2 Bytes ] ...                       |
//! +--------------------------------------------------------------------------------+
//! ```
//!
//! At read time, [`unpack_toc`] parses the header and TOC without copying the file payloads,
//! allowing [`super::directory::SharedMemoryDirectory`] to provide zero-copy [`tantivy::directory::FileSlice`]
//! references directly into PostgreSQL shared memory.

use std::collections::HashMap;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    AntiCallToken, Directory, FileHandle, FileSlice, InnerWritePtr, Lock, TempFilePtr,
    TerminatingWrite, WatchCallback, WatchHandle,
};
use tantivy::index::SegmentId;

/// Magic number identifying a packed segment buffer ("PDBM").
pub const PACKED_MAGIC: u32 = 0x5044424D;
/// Version of the packed segment format.
pub const PACKED_VERSION: u32 = 1;
/// Maximum length of a component file name in bytes.
pub const MAX_FILENAME_LEN: usize = 62;

/// Fixed-size header at the beginning of a packed segment buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PackedHeader {
    /// Magic identifier ([`PACKED_MAGIC`]).
    pub magic: u32,
    /// Format version ([`PACKED_VERSION`]).
    pub version: u32,
    /// Number of component files stored in the TOC.
    pub file_count: u32,
    /// Reserved for alignment and future extensions.
    pub _reserved: u32,
    /// Total byte length of the packed segment buffer including header and all files.
    pub total_bytes: u64,
}

/// Table of Contents entry describing a single component file.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PackedTocEntry {
    /// Length of the file name in bytes.
    pub name_len: u16,
    /// UTF-8 encoded file name bytes.
    pub name: [u8; MAX_FILENAME_LEN],
    /// Absolute byte offset of the file payload within the packed buffer (8-byte aligned).
    pub offset: u64,
    /// Exact byte length of the file payload.
    pub len: u64,
}

// TODO: Once `RamDirectory` in Tantivy implements `list_managed_files()` or exposes internal file
// iteration, `pack_segment` can read `FileSlice`s directly without calling `RamDirectory::persist`
// or using `SegmentCollector`, eliminating intermediate heap allocations and buffer copies entirely.

type CollectedFiles = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A directory implementation that collects written files matching a prefix into memory buffers.
#[derive(Default, Clone, Debug)]
pub struct SegmentCollector {
    prefix: String,
    files: CollectedFiles,
}

impl SegmentCollector {
    pub fn new(prefix: String) -> Self {
        Self {
            prefix,
            files: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn into_files(self) -> Vec<(String, Vec<u8>)> {
        Arc::try_unwrap(self.files)
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_else(|arc| arc.lock().unwrap().clone())
    }
}

struct CollectorWriter {
    filename: String,
    files: CollectedFiles,
    data: Vec<u8>,
}

impl Write for CollectorWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for CollectorWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        files.push((self.filename.clone(), std::mem::take(&mut self.data)));
        Ok(())
    }
}

struct DiscardWriter;

impl Write for DiscardWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TerminatingWrite for DiscardWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> io::Result<()> {
        Ok(())
    }
}

impl Directory for SegmentCollector {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        Err(OpenReadError::FileDoesNotExist(path.to_path_buf()))
    }

    fn open_read(&self, path: &Path) -> Result<FileSlice, OpenReadError> {
        Err(OpenReadError::FileDoesNotExist(path.to_path_buf()))
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        Err(DeleteError::FileDoesNotExist(path.to_path_buf()))
    }

    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(false)
    }

    fn open_write_inner(&self, path: &Path) -> Result<InnerWritePtr, OpenWriteError> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if filename.starts_with(&self.prefix) {
            Ok(Box::new(CollectorWriter {
                filename,
                files: Arc::clone(&self.files),
                data: Vec::new(),
            }))
        } else {
            Ok(Box::new(DiscardWriter))
        }
    }

    fn open_temp_file(&self) -> io::Result<TempFilePtr> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "temp file unsupported in SegmentCollector",
        ))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        Err(OpenReadError::FileDoesNotExist(path.to_path_buf()))
    }

    fn atomic_write(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic write unsupported in SegmentCollector",
        ))
    }

    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn acquire_lock(&self, _lock: &Lock) -> Result<tantivy::directory::DirectoryLock, LockError> {
        Err(LockError::LockBusy)
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        Ok(WatchHandle::empty())
    }
}

/// Pack all files in a `RamDirectory` belonging to `segment_id` into a single binary buffer.
pub fn pack_segment(
    ram_directory: &tantivy::directory::RamDirectory,
    segment_id: &SegmentId,
) -> anyhow::Result<Vec<u8>> {
    let id_str = segment_id.uuid_string();
    let collector = SegmentCollector::new(id_str);
    ram_directory.persist(&collector)?;
    let mut relevant_files = collector.into_files();
    relevant_files.sort_by(|a, b| a.0.cmp(&b.0));

    let file_count = relevant_files.len();
    let header_size = std::mem::size_of::<PackedHeader>();
    let toc_entry_size = std::mem::size_of::<PackedTocEntry>();
    let toc_size = header_size + file_count * toc_entry_size;

    let mut current_offset = (toc_size + 7) & !7;
    let mut toc_entries = Vec::with_capacity(file_count);

    for (name, data) in &relevant_files {
        let name_bytes = name.as_bytes();
        if name_bytes.len() > MAX_FILENAME_LEN {
            anyhow::bail!(
                "segment file name '{}' exceeds max length {}",
                name,
                MAX_FILENAME_LEN
            );
        }
        let mut name_arr = [0u8; MAX_FILENAME_LEN];
        name_arr[..name_bytes.len()].copy_from_slice(name_bytes);

        let aligned_offset = (current_offset + 7) & !7;
        let len = data.len() as u64;

        toc_entries.push(PackedTocEntry {
            name_len: name_bytes.len() as u16,
            name: name_arr,
            offset: aligned_offset as u64,
            len,
        });

        current_offset = aligned_offset + data.len();
    }

    let total_bytes = ((current_offset + 7) & !7) as u64;
    let mut buffer = vec![0u8; total_bytes as usize];

    let header = PackedHeader {
        magic: PACKED_MAGIC,
        version: PACKED_VERSION,
        file_count: file_count as u32,
        _reserved: 0,
        total_bytes,
    };

    unsafe {
        std::ptr::copy_nonoverlapping(
            &header as *const PackedHeader as *const u8,
            buffer.as_mut_ptr(),
            header_size,
        );

        let toc_ptr = buffer.as_mut_ptr().add(header_size) as *mut PackedTocEntry;
        for (i, entry) in toc_entries.iter().enumerate() {
            std::ptr::copy_nonoverlapping(entry as *const PackedTocEntry, toc_ptr.add(i), 1);
        }
    }

    for (i, (_, data)) in relevant_files.iter().enumerate() {
        let offset = toc_entries[i].offset as usize;
        buffer[offset..offset + data.len()].copy_from_slice(data);
    }

    Ok(buffer)
}

/// Unpack the Table of Contents from a packed segment buffer.
pub fn unpack_toc(slice: &[u8]) -> Option<HashMap<PathBuf, Range<usize>>> {
    let header_size = std::mem::size_of::<PackedHeader>();
    if slice.len() < header_size {
        return None;
    }

    let header = unsafe { std::ptr::read_unaligned(slice.as_ptr() as *const PackedHeader) };
    if header.magic != PACKED_MAGIC || header.version != PACKED_VERSION {
        return None;
    }

    let file_count = usize::try_from(header.file_count).ok()?;
    let toc_entry_size = std::mem::size_of::<PackedTocEntry>();
    let total_toc_size = header_size.checked_add(file_count.checked_mul(toc_entry_size)?)?;
    let total_bytes = usize::try_from(header.total_bytes).ok()?;

    if slice.len() < total_toc_size || slice.len() < total_bytes {
        return None;
    }

    let mut map = HashMap::with_capacity(file_count);
    let toc_ptr = unsafe { slice.as_ptr().add(header_size) as *const PackedTocEntry };

    for i in 0..file_count {
        let entry = unsafe { std::ptr::read_unaligned(toc_ptr.add(i)) };
        let name_len = usize::from(entry.name_len);
        if name_len > MAX_FILENAME_LEN {
            return None;
        }

        let name_str = std::str::from_utf8(&entry.name[..name_len]).ok()?;
        let start = usize::try_from(entry.offset).ok()?;
        let len = usize::try_from(entry.len).ok()?;
        let end = start.checked_add(len)?;

        if start < total_toc_size || end > slice.len() {
            return None;
        }

        map.insert(PathBuf::from(name_str), start..end);
    }

    Some(map)
}
