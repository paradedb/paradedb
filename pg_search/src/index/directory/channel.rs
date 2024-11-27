use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use parking_lot::Mutex;
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, io::Write, ops::Range, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    DirectoryLock, FileHandle, Lock, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
};
use tantivy::Directory;

use crate::index::directory::blocking::{BlockingDirectory, BlockingLock};
use crate::index::reader::channel::ChannelReader;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::channel::ChannelWriter;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{bm25_max_free_space, DirectoryEntry};

#[derive(Debug)]
pub enum ChannelRequest {
    AcquireLock(Lock),
    AtomicRead(PathBuf),
    AtomicWrite(PathBuf, Vec<u8>),
    ListManagedFiles(),
    RegisterFilesAsManaged(Vec<PathBuf>, bool),
    ReleaseBlockingLock(BlockingLock),
    SegmentRead(Range<usize>, DirectoryEntry),
    SegmentWrite(PathBuf, Vec<u8>),
    SegmentWriteTerminate(PathBuf),
    SegmentDelete(PathBuf),
    GetSegmentComponent(PathBuf),
    Terminate,
}

pub enum ChannelResponse {
    ManagedFiles(HashSet<PathBuf>),
    AcquiredLock(BlockingLock),
    Bytes(Vec<u8>),
    DirectoryEntry(DirectoryEntry),
}

impl Debug for ChannelResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelResponse::AcquiredLock(_) => write!(f, "AcquiredLock"),
            ChannelResponse::ManagedFiles(_) => write!(f, "ManagedFiles"),
            ChannelResponse::Bytes(_) => write!(f, "Bytes"),
            ChannelResponse::DirectoryEntry(_) => write!(f, "DirectoryEntry"),
        }
    }
}

pub struct ChannelLock {
    // This is an Option because we need to take ownership of the lock in the Drop implementation
    lock: Option<BlockingLock>,
    sender: Sender<ChannelRequest>,
}

impl Drop for ChannelLock {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            self.sender
                .send(ChannelRequest::ReleaseBlockingLock(lock))
                .expect("should be able to release channel lock");
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChannelDirectory {
    sender: Sender<ChannelRequest>,
    receiver: Receiver<ChannelResponse>,
}

// A directory that actually forwards all read/write requests to a channel
// This channel is used to communicate with the actual storage implementation
impl ChannelDirectory {
    pub fn new(sender: Sender<ChannelRequest>, receiver: Receiver<ChannelResponse>) -> Self {
        Self { sender, receiver }
    }
}

impl Directory for ChannelDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        Ok(Arc::new(unsafe {
            ChannelReader::new(path, self.sender.clone(), self.receiver.clone()).map_err(|e| {
                OpenReadError::wrap_io_error(
                    io::Error::new(io::ErrorKind::Other, format!("{:?}", e)),
                    path.to_path_buf(),
                )
            })?
        }))
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        Ok(io::BufWriter::with_capacity(
            unsafe { bm25_max_free_space() },
            Box::new(unsafe { ChannelWriter::new(path, self.sender.clone()) }),
        ))
    }

    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        self.sender
            .send(ChannelRequest::AtomicRead(path.to_path_buf()))
            .unwrap();

        match self.receiver.recv().unwrap() {
            ChannelResponse::Bytes(bytes) => Ok(bytes),
            unexpected => Err(OpenReadError::wrap_io_error(
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("atomic_read unexpected response {:?}", unexpected),
                ),
                path.to_path_buf(),
            )),
        }
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        self.sender
            .send(ChannelRequest::AtomicWrite(
                path.to_path_buf(),
                data.to_vec(),
            ))
            .unwrap();

        Ok(())
    }

    fn delete(&self, path: &Path) -> result::Result<(), DeleteError> {
        self.sender
            .send(ChannelRequest::SegmentDelete(path.to_path_buf()))
            .unwrap();

        Ok(())
    }

    // Internally, Tantivy only uses this for meta.json, which should always exist
    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(true)
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        self.sender
            .send(ChannelRequest::AcquireLock(Lock {
                filepath: lock.filepath.clone(),
                is_blocking: lock.is_blocking,
            }))
            .unwrap();

        match self.receiver.recv().unwrap() {
            ChannelResponse::AcquiredLock(blocking_lock) => {
                Ok(DirectoryLock::from(Box::new(ChannelLock {
                    lock: Some(blocking_lock),
                    sender: self.sender.clone(),
                })))
            }
            unexpected => Err(LockError::IoError(
                io::Error::new(
                    io::ErrorKind::Other,
                    format!("acquire_lock unexpected response {:?}", unexpected),
                )
                .into(),
            )),
        }
    }

    // Internally, tantivy only uses this API to detect new commits to implement the
    // `OnCommitWithDelay` `ReloadPolicy`. Not implementing watch in a `Directory` only prevents
    // the `OnCommitWithDelay` `ReloadPolicy` to work properly.
    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        unimplemented!("OnCommitWithDelay ReloadPolicy not supported");
    }

    // Block storage handles disk writes for us, we don't need to fsync
    fn sync_directory(&self) -> io::Result<()> {
        Ok(())
    }

    fn list_managed_files(&self) -> tantivy::Result<HashSet<PathBuf>> {
        self.sender
            .send(ChannelRequest::ListManagedFiles())
            .unwrap();

        match self.receiver.recv().unwrap() {
            ChannelResponse::ManagedFiles(files) => Ok(files),
            unexpected => Err(tantivy::TantivyError::ErrorInThread(format!(
                "list_managed_files unexpected response {:?}",
                unexpected
            ))),
        }
    }

    fn register_files_as_managed(
        &self,
        files: Vec<PathBuf>,
        overwrite: bool,
    ) -> tantivy::Result<()> {
        self.sender
            .send(ChannelRequest::RegisterFilesAsManaged(files, overwrite))
            .unwrap();

        Ok(())
    }
}

type Action = Box<dyn FnOnce() -> Reply + Send + Sync>;
type Reply = Box<dyn Any + Send + Sync>;
pub struct ChannelRequestHandler {
    directory: BlockingDirectory,
    relation_oid: pg_sys::Oid,
    sender: Sender<ChannelResponse>,
    receiver: Receiver<ChannelRequest>,
    writers: Mutex<FxHashMap<PathBuf, SegmentComponentWriter>>,
    readers: Mutex<FxHashMap<PathBuf, SegmentComponentReader>>,

    action: (Sender<Action>, Receiver<Action>),
    reply: (Sender<Reply>, Receiver<Reply>),
    _worker: JoinHandle<()>,
}

#[derive(Debug)]
pub struct ChannelRequestStats {
    pub deleted_paths: Vec<PathBuf>,
}

pub type ShouldTerminate = bool;

impl ChannelRequestHandler {
    pub fn open(
        directory: BlockingDirectory,
        relation_oid: pg_sys::Oid,
        sender: Sender<ChannelResponse>,
        receiver: Receiver<ChannelRequest>,
    ) -> Self {
        let (action_sender, action_receiver) = crossbeam::channel::bounded(1);
        let (reply_sender, reply_receiver) = crossbeam::channel::bounded(1);
        Self {
            directory,
            relation_oid,
            receiver,
            sender,
            writers: Default::default(),
            readers: Default::default(),
            action: (action_sender, action_receiver.clone()),
            reply: (reply_sender.clone(), reply_receiver),
            _worker: std::thread::spawn(move || {
                for message in action_receiver {
                    if reply_sender.send(message()).is_err() {
                        // channel was dropped and that's okay
                        break;
                    }
                }
            }),
        }
    }

    pub fn wait_for<T: Send + Sync + 'static, F: FnOnce() -> T + Send + Sync + 'static>(
        &self,
        func: F,
    ) -> Result<T> {
        let func: Action = Box::new(move || Box::new(func()));
        self.action.0.send(func)?;
        loop {
            match self.reply.1.try_recv() {
                // `func` has finished and we have its reply
                Ok(reply) => {
                    return match reply.downcast::<T>() {
                        // the reply is exactly what we hoped for
                        Ok(reply) => Ok(*reply),

                        // it's something else, so transform into a generic error
                        Err(e) => Err(anyhow::anyhow!("unexpected reply {:?}", e)),
                    };
                }

                // we have no reply yet, so process any messages it may have generated
                Err(TryRecvError::Empty) => {
                    for message in self.receiver.try_iter() {
                        match self.process_message(message, &mut vec![]) {
                            Ok(should_terminate) if should_terminate => break,
                            Ok(_) => continue,
                            Err(e) => return Err(e),
                        }
                    }
                }

                // the reply channel was closed, so lets just return that as the error
                Err(TryRecvError::Disconnected) => {
                    return Err(anyhow::anyhow!("reply channel disconnected"))
                }
            }
        }
    }

    pub fn receive_blocking(&self) -> Result<ChannelRequestStats> {
        let mut deleted_paths: Vec<PathBuf> = vec![];

        let receiver = self.receiver.clone();
        for message in receiver.into_iter() {
            self.process_message(message, &mut deleted_paths)?;
        }

        Ok(ChannelRequestStats { deleted_paths })
    }

    fn process_message(
        &self,
        message: ChannelRequest,
        deleted_paths: &mut Vec<PathBuf>,
    ) -> Result<ShouldTerminate> {
        match message {
            ChannelRequest::AcquireLock(lock) => {
                let blocking_lock = unsafe { self.directory.acquire_blocking_lock(&lock)? };
                self.sender
                    .send(ChannelResponse::AcquiredLock(blocking_lock))?;
            }
            ChannelRequest::AtomicRead(path) => {
                let data = self.directory.atomic_read(&path)?;
                self.sender.send(ChannelResponse::Bytes(data))?;
            }
            ChannelRequest::AtomicWrite(path, data) => {
                self.directory.atomic_write(&path, &data)?;
            }
            ChannelRequest::ListManagedFiles() => {
                let managed_files = self.directory.list_managed_files()?;
                self.sender
                    .send(ChannelResponse::ManagedFiles(managed_files))?;
            }
            ChannelRequest::RegisterFilesAsManaged(files, overwrite) => {
                self.directory.register_files_as_managed(files, overwrite)?;
            }
            ChannelRequest::GetSegmentComponent(path) => {
                let (opaque, _, _) = unsafe { self.directory.directory_lookup(&path)? };
                self.sender.send(ChannelResponse::DirectoryEntry(opaque))?;
            }
            ChannelRequest::ReleaseBlockingLock(blocking_lock) => {
                drop(blocking_lock);
            }
            ChannelRequest::SegmentRead(range, handle) => {
                let mut mutex = self.readers.lock();
                let reader = mutex.entry(handle.path.clone()).or_insert_with(|| unsafe {
                    SegmentComponentReader::new(self.relation_oid, handle)
                });
                let data = reader.read_bytes(range)?;
                drop(mutex);

                self.sender
                    .send(ChannelResponse::Bytes(data.as_slice().to_owned()))?;
            }
            ChannelRequest::SegmentWrite(path, data) => {
                let mut mutex = self.writers.lock();
                let writer = mutex.entry(path.clone()).or_insert_with(|| unsafe {
                    SegmentComponentWriter::new(self.relation_oid, &path)
                });
                writer.write_all(&data)?;
            }
            ChannelRequest::SegmentWriteTerminate(path) => {
                let mut mutex = self.writers.lock();
                let writer = mutex.remove(&path).expect("writer should exist");
                writer.terminate()?;
            }
            ChannelRequest::SegmentDelete(path) => {
                if (self.directory.try_delete(&path)?).is_some() {
                    deleted_paths.push(path);
                }
            }
            ChannelRequest::Terminate => {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
