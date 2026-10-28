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

use crate::api::{HashMap, HashSet};
use crate::index::directory::mvcc::PgTempFile;
use crate::postgres::buffile::{BufFileReleaseGuard, TempFileOwner};
use crate::postgres::storage::block::SegmentFileDetails;
use parking_lot::Mutex;
use pgrx::pg_sys;
use std::fmt;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    AntiCallToken, DirectoryLock, FileHandle, InnerWritePtr, Lock, OwnedBytes, TempFilePtr,
    TerminatingWrite, WatchCallback, WatchHandle,
};
use tantivy::index::SegmentId;
use tantivy::{Directory, HasLen};

type SharedFile = Arc<Mutex<BuildFile>>;

/// Worker-local segment components. Only final build outputs enter the index relation.
#[derive(Clone)]
pub(crate) struct BuildDirectory {
    files: Arc<Mutex<HashMap<PathBuf, SharedFile>>>,
    release_guard: Arc<BufFileReleaseGuard>,
    #[cfg(unix)]
    temp_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

impl fmt::Debug for BuildDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuildDirectory")
            .field("files", &self.files.lock().len())
            .finish()
    }
}

impl BuildDirectory {
    pub fn new() -> Self {
        Self {
            files: Default::default(),
            release_guard: BufFileReleaseGuard::register(),
            #[cfg(unix)]
            temp_paths: Default::default(),
        }
    }

    pub fn segment_byte_size(&self, segment_id: SegmentId) -> u64 {
        self.files
            .lock()
            .iter()
            .filter(|(path, _)| path.segment_id() == Some(segment_id))
            .map(|(_, file)| file.lock().len as u64)
            .sum()
    }

    pub fn available_disk_bytes(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            self.temp_paths
                .lock()
                .iter()
                .filter_map(|path| rustix::fs::statvfs(path).ok())
                .map(|stat| stat.f_bavail.saturating_mul(stat.f_frsize))
                .min()
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    pub fn segment_files(&self, segment_ids: impl IntoIterator<Item = SegmentId>) -> Vec<PathBuf> {
        let ids: HashSet<_> = segment_ids.into_iter().collect();
        self.files
            .lock()
            .keys()
            .filter(|path| path.segment_id().is_some_and(|id| ids.contains(&id)))
            .cloned()
            .collect()
    }
}

impl Directory for BuildDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let file = self
            .files
            .lock()
            .get(path)
            .cloned()
            .ok_or_else(|| OpenReadError::FileDoesNotExist(path.to_path_buf()))?;
        let len = file.lock().len;
        Ok(Arc::new(BuildFileReader { file, len }))
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        self.files
            .lock()
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| DeleteError::FileDoesNotExist(path.to_path_buf()))
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        Ok(self.files.lock().contains_key(path))
    }

    fn open_write_inner(&self, path: &Path) -> Result<InnerWritePtr, OpenWriteError> {
        let mut files = self.files.lock();
        if files.contains_key(path) {
            return Err(OpenWriteError::FileAlreadyExists(path.to_path_buf()));
        }
        assert_eq!(self.release_guard.owner(), unsafe {
            pg_sys::CurrentResourceOwner
        });
        let file = Arc::new(Mutex::new(BuildFile {
            file: TempFileOwner::new(
                unsafe {
                    pg_sys::PrepareTempTablespaces();
                    pg_sys::OpenTemporaryFile(false)
                },
                self.release_guard.clone(),
            ),
            len: 0,
        }));
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let directory = file
                .lock()
                .file
                .with(|handle| {
                    let name = unsafe { std::ffi::CStr::from_ptr(pg_sys::FilePathName(handle)) };
                    Ok(Path::new(std::ffi::OsStr::from_bytes(name.to_bytes()))
                        .parent()
                        .expect("temporary file directory")
                        .to_path_buf())
                })
                .map_err(|error| OpenWriteError::wrap_io_error(error, path.to_path_buf()))?;
            self.temp_paths.lock().insert(directory);
        }
        files.insert(path.to_path_buf(), file.clone());
        Ok(Box::new(BuildFileWriter(file)))
    }

    fn open_temp_file(&self) -> io::Result<TempFilePtr> {
        Ok(Box::new(PgTempFile::create()))
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        Err(OpenReadError::FileDoesNotExist(path.to_path_buf()))
    }

    fn atomic_write(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "build directories only store segment components",
        ))
    }

    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn acquire_lock(&self, _lock: &Lock) -> Result<DirectoryLock, LockError> {
        Ok(DirectoryLock::from(Box::new(())))
    }

    fn watch(&self, _callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        Err(tantivy::TantivyError::InvalidArgument(
            "build directories do not support commit watches".into(),
        ))
    }

    fn list_managed_files(&self) -> tantivy::Result<std::collections::HashSet<PathBuf>> {
        Ok(self.files.lock().keys().cloned().collect())
    }

    fn register_files_as_managed(
        &self,
        _files: Vec<PathBuf>,
        _overwrite: bool,
    ) -> tantivy::Result<()> {
        Ok(())
    }

    fn supports_garbage_collection(&self) -> bool {
        false
    }
}

struct BuildFile {
    file: TempFileOwner<pg_sys::File>,
    len: usize,
}

impl BuildFile {
    fn read_exact_at(&self, data: &mut [u8], offset: usize) -> io::Result<()> {
        let mut offset: pg_sys::off_t = offset.try_into().map_err(io::Error::other)?;
        pgrx::check_for_interrupts!();
        self.file.with(|file| {
            let mut data = data;
            while !data.is_empty() {
                let n = unsafe {
                    pg_sys::FileRead(
                        file,
                        data.as_mut_ptr().cast(),
                        data.len().min(i32::MAX as usize) as _,
                        offset,
                        pg_sys::WaitEventIO::WAIT_EVENT_BUFFILE_READ,
                    )
                };
                if n < 0 {
                    return Err(io::Error::last_os_error());
                }
                if n == 0 {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                offset += n as pg_sys::off_t;
                data = &mut data[n as usize..];
            }
            Ok(())
        })
    }
}

struct BuildFileWriter(SharedFile);

impl Write for BuildFileWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        // Other component BufWriters may flush on drop while a PostgreSQL error unwinds.
        if std::thread::panicking() {
            return Ok(data.len());
        }
        let mut file = self.0.lock();
        let offset = file.len;
        let new_len = offset.checked_add(data.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "build temporary file is too large",
            )
        })?;
        let _: pg_sys::off_t = new_len.try_into().map_err(io::Error::other)?;
        pgrx::check_for_interrupts!();
        file.file.with(|handle| {
            let mut offset = offset as pg_sys::off_t;
            let mut remaining = data;
            while !remaining.is_empty() {
                let n = unsafe {
                    pg_sys::FileWrite(
                        handle,
                        remaining.as_ptr().cast_mut().cast(),
                        remaining.len().min(i32::MAX as usize) as _,
                        offset,
                        pg_sys::WaitEventIO::WAIT_EVENT_BUFFILE_WRITE,
                    )
                };
                if n < 0 {
                    return Err(io::Error::last_os_error());
                }
                if n == 0 {
                    return Err(io::ErrorKind::WriteZero.into());
                }
                offset += n as pg_sys::off_t;
                remaining = &remaining[n as usize..];
            }
            Ok(())
        })?;
        file.len = new_len;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Writes go directly to the VFD; scratch data does not need durable flushing.
        Ok(())
    }
}

impl TerminatingWrite for BuildFileWriter {
    fn terminate_ref(&mut self, _token: AntiCallToken) -> io::Result<()> {
        self.flush()
    }
}

struct BuildFileReader {
    file: SharedFile,
    len: usize,
}

impl fmt::Debug for BuildFileReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuildFileReader")
            .field("len", &self.len)
            .finish()
    }
}

impl HasLen for BuildFileReader {
    fn len(&self) -> usize {
        self.len
    }
}

impl FileHandle for BuildFileReader {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        let end = range.end.min(self.len);
        if range.start > end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid file range",
            ));
        }
        let mut data = vec![0; end - range.start];
        let file = self.file.lock();
        file.read_exact_at(&mut data, range.start)?;
        Ok(OwnedBytes::new(data))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_build_directory_interleaved_io_and_delete() {
        let directory = BuildDirectory::new();
        let path = Path::new("component.fast");
        let data: Vec<_> = (0..pg_sys::BLCKSZ as usize * 3).map(|i| i as u8).collect();
        let mut writer = directory.open_write(path).unwrap();
        writer.write_all(&data).unwrap();
        writer.flush().unwrap();
        let reader = directory.clone().open_read(path).unwrap();
        assert_eq!(
            reader.read_bytes_slice(8000..9000).unwrap().as_ref(),
            &data[8000..9000]
        );
        writer.write_all(b"tail").unwrap();
        writer.terminate().unwrap();
        assert_eq!(reader.read_bytes().unwrap().as_ref(), data);
        let final_reader = directory.open_read(path).unwrap();
        assert_eq!(
            final_reader
                .read_bytes_slice(data.len()..data.len() + 4)
                .unwrap()
                .as_ref(),
            b"tail"
        );
        directory.delete(path).unwrap();
        assert!(!directory.exists(path).unwrap());
        assert_eq!(
            reader.read_bytes_slice(0..32).unwrap().as_ref(),
            &data[..32]
        );
    }

    #[pg_test]
    fn test_build_directory_many_components_without_resident_buffers() {
        let directory = BuildDirectory::new();
        let memory_before = Spi::get_one::<i64>(
            "SELECT total_bytes FROM pg_backend_memory_contexts WHERE name = 'TopTransactionContext'",
        ).unwrap().unwrap();
        let components = 1200_i64;
        for i in 0..components {
            let mut writer = directory
                .open_write(Path::new(&format!("{i}.fast")))
                .unwrap();
            writer.write_all(&i.to_le_bytes()).unwrap();
            writer.terminate().unwrap();
        }
        let memory_after = Spi::get_one::<i64>(
            "SELECT total_bytes FROM pg_backend_memory_contexts WHERE name = 'TopTransactionContext'",
        ).unwrap().unwrap();
        assert!(memory_after - memory_before < components * pg_sys::BLCKSZ as i64 / 2);
        for i in 0..components {
            let path = PathBuf::from(format!("{i}.fast"));
            assert_eq!(
                directory
                    .open_read(&path)
                    .unwrap()
                    .read_bytes()
                    .unwrap()
                    .as_ref(),
                i.to_le_bytes()
            );
            directory.delete(&path).unwrap();
        }
    }

    #[pg_test]
    fn test_build_directory_crosses_physical_file_boundary() {
        Spi::run("SET LOCAL temp_file_limit = '-1'").unwrap();
        let directory = BuildDirectory::new();
        let path = Path::new("large.fast");
        let mut writer = directory.open_write(path).unwrap();
        let boundary = 1 << 30;
        directory.files.lock().get(path).unwrap().lock().len = boundary - 4;
        writer.write_all(b"abcdefgh").unwrap();
        writer.terminate().unwrap();
        let reader = directory.open_read(path).unwrap();
        assert_eq!(
            reader
                .read_bytes_slice(boundary - 4..boundary + 4)
                .unwrap()
                .as_ref(),
            b"abcdefgh"
        );
        assert_eq!(
            reader
                .read_bytes_slice(boundary..boundary + 4)
                .unwrap()
                .as_ref(),
            b"efgh"
        );
    }

    #[pg_test]
    #[should_panic(expected = "temporary file size exceeds")]
    fn test_build_directory_preserves_temp_limit_error() {
        Spi::run("SET LOCAL temp_file_limit = '1kB'").unwrap();
        let directory = BuildDirectory::new();
        let mut buffered = directory.open_write(Path::new("buffered.fast")).unwrap();
        buffered.write_all(b"buffered").unwrap();
        let mut writer = directory.open_write(Path::new("overflow.fast")).unwrap();
        writer.write_all(&[0; pg_sys::BLCKSZ as usize * 2]).unwrap();
    }
}
