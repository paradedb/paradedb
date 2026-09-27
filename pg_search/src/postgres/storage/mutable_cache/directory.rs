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

//! Read-only Tantivy [`Directory`] backed by contiguous PostgreSQL shared memory.
//!
//! # Architecture
//!
//! Tantivy requires each component file (`.term`, `.idx`, `.fast`, etc.) to be presented as a
//! [`tantivy::directory::FileSlice`]. By storing the segment packed contiguously with a Table of Contents (TOC)
//! in a shared-memory slab allocated from [`super::slab_pool::SlabPool`], [`SharedMemoryDirectory`] implements
//! [`tantivy::Directory`] by creating sub-slices into the shared-memory arena without copying or serialization.
//!
//! Slices are wrapped in [`SharedMemorySlice`], which carries an [`Arc<ActiveReaderGuard>`]. As long as any
//! file slice or directory handle is held by query execution or scoring, the slot's reader refcount remains positive.
//! When all slices are dropped, [`super::ActiveReaderGuard::drop`] executes, immediately freeing superseded slabs
//! back to the slab pool.

use stable_deref_trait::StableDeref;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::ops::{Deref, Range};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    Directory, FileHandle, FileSlice, InnerWritePtr, Lock, OwnedBytes, TempFilePtr, WatchCallback,
    WatchHandle,
};

use super::ActiveReaderGuard;

/// Zero-copy byte slice pointing into shared memory, guarded by an active-reader refcount.
///
/// Implements [`Deref<Target = [u8]>`] and [`StableDeref`], allowing Tantivy's [`OwnedBytes`]
/// to treat it as an owned buffer with a stable address.
#[derive(Clone)]
pub struct SharedMemorySlice {
    ptr: *const u8,
    len: usize,
    _guard: Arc<ActiveReaderGuard>,
}

// SAFETY: The underlying memory is allocated in PostgreSQL shared memory and remains valid
// as long as `_guard` keeps the slot's `active_readers` count > 0.
unsafe impl Send for SharedMemorySlice {}
unsafe impl Sync for SharedMemorySlice {}

impl Deref for SharedMemorySlice {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

unsafe impl StableDeref for SharedMemorySlice {}

impl fmt::Debug for SharedMemorySlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedMemorySlice")
            .field("len", &self.len)
            .finish()
    }
}

/// A read-only Tantivy [`Directory`] implementation over a packed segment in shared memory.
///
/// Maps file paths to byte ranges in a contiguous shared-memory slab using an in-memory Table of Contents.
/// Provides zero-copy [`FileSlice`] references directly into PostgreSQL shared memory.
#[derive(Clone)]
pub struct SharedMemoryDirectory {
    slice: SharedMemorySlice,
    toc: Arc<HashMap<PathBuf, Range<usize>>>,
}

impl SharedMemoryDirectory {
    pub fn new(
        ptr: *const u8,
        len: usize,
        guard: Arc<ActiveReaderGuard>,
        toc: HashMap<PathBuf, Range<usize>>,
    ) -> Self {
        Self {
            slice: SharedMemorySlice {
                ptr,
                len,
                _guard: guard,
            },
            toc: Arc::new(toc),
        }
    }

    fn resolve_path<'a>(&'a self, path: &'a Path) -> Option<&'a Range<usize>> {
        self.toc.get(path).or_else(|| {
            path.file_name()
                .map(Path::new)
                .and_then(|name| self.toc.get(name))
        })
    }
}

impl fmt::Debug for SharedMemoryDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedMemoryDirectory")
            .field("file_count", &self.toc.len())
            .field("len", &self.slice.len)
            .finish()
    }
}

impl Directory for SharedMemoryDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let file_slice = self.open_read(path)?;
        Ok(Arc::new(file_slice))
    }

    fn open_read(&self, path: &Path) -> Result<FileSlice, OpenReadError> {
        let range = self
            .resolve_path(path)
            .ok_or_else(|| OpenReadError::FileDoesNotExist(path.to_path_buf()))?;

        let owned_bytes = OwnedBytes::new(self.slice.clone());
        let file_slice = FileSlice::new(Arc::new(owned_bytes)).slice(range.clone());
        Ok(file_slice)
    }

    fn delete(&self, _path: &Path) -> Result<(), DeleteError> {
        Ok(())
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        Ok(self.resolve_path(path).is_some())
    }

    fn open_write_inner(&self, path: &Path) -> Result<InnerWritePtr, OpenWriteError> {
        Err(OpenWriteError::IoError {
            io_error: Arc::new(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "SharedMemoryDirectory is read-only: cannot write to {:?}",
                    path
                ),
            )),
            filepath: path.to_path_buf(),
        })
    }

    fn open_temp_file(&self) -> io::Result<TempFilePtr> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "SharedMemoryDirectory is read-only",
        ))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let bytes =
            self.open_read(path)?
                .read_bytes()
                .map_err(|io_error| OpenReadError::IoError {
                    io_error: Arc::new(io_error),
                    filepath: path.to_path_buf(),
                })?;
        Ok(bytes.as_slice().to_vec())
    }

    fn atomic_write(&self, path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "SharedMemoryDirectory is read-only: cannot write to {:?}",
                path
            ),
        ))
    }

    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn acquire_lock(&self, lock: &Lock) -> Result<tantivy::directory::DirectoryLock, LockError> {
        Ok(tantivy::directory::DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        Ok(WatchHandle::empty())
    }

    fn list_managed_files(&self) -> tantivy::Result<HashSet<PathBuf>> {
        Ok(self.toc.keys().cloned().collect())
    }
}
