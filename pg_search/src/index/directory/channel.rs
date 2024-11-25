use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{io, io::Write, ops::Range, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    DirectoryLock, FileHandle, Lock, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
};
use tantivy::index::SegmentMetaInventory;
use tantivy::{Directory, IndexMeta};

use crate::index::directory::blocking::BlockingDirectory;
use crate::index::reader::channel::ChannelReader;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::channel::ChannelWriter;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{bm25_max_free_space, DirectoryEntry};

pub enum ChannelRequest {
    ListManagedFiles(),
    RegisterFilesAsManaged(Vec<PathBuf>, bool),
    SegmentRead(Range<usize>, DirectoryEntry),
    SegmentWrite(PathBuf, Vec<u8>),
    SegmentWriteTerminate(PathBuf),
    GetSegmentComponent(PathBuf),
    ShouldDeleteCtids(Vec<u64>),
    SaveMetas(IndexMeta),
    LoadMetas(SegmentMetaInventory),
    Terminate,
}

pub enum ChannelResponse {
    ManagedFiles(HashSet<PathBuf>),
    Bytes(Vec<u8>),
    DirectoryEntry(DirectoryEntry),
    ShouldDeleteCtids(Vec<u64>),
    LoadMetas(IndexMeta),
}

impl Debug for ChannelResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelResponse::ManagedFiles(_) => write!(f, "ManagedFiles"),
            ChannelResponse::Bytes(_) => write!(f, "Bytes"),
            ChannelResponse::DirectoryEntry(_) => write!(f, "DirectoryEntry"),
            ChannelResponse::ShouldDeleteCtids(_) => write!(f, "ShouldDeleteCtids"),
            ChannelResponse::LoadMetas(_) => write!(f, "LoadMetas"),
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

    /// atomic_write is used by Tantivy to write to managed.json, meta.json, and create .lock files
    /// This function should never be called by our Tantivy fork because we write to managed.json and meta.json ourselves
    fn atomic_write(&self, path: &Path, _data: &[u8]) -> io::Result<()> {
        unimplemented!("atomic_write should not be called for {:?}", path);
    }

    /// atomic_read is used by Tantivy to read from managed.json and meta.json
    /// This function should never be called by our Tantivy fork because we read from them ourselves
    fn atomic_read(&self, path: &Path) -> result::Result<Vec<u8>, OpenReadError> {
        unimplemented!("atomic_read should not be called for {:?}", path);
    }

    // This is called by Tantivy's garbage collect process, which we do not want to implement
    // because we use Postgres MVCC rules for our own garbage collection in amvacuumcleanup
    fn delete(&self, _path: &Path) -> result::Result<(), DeleteError> {
        Ok(())
    }

    // Internally, Tantivy only uses this for meta.json, which should always exist
    fn exists(&self, _path: &Path) -> Result<bool, OpenReadError> {
        Ok(true)
    }

    fn acquire_lock(&self, lock: &Lock) -> result::Result<DirectoryLock, LockError> {
        Ok(DirectoryLock::from(Box::new(Lock {
            filepath: lock.filepath.clone(),
            is_blocking: true,
        })))
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

    fn save_metas(&self, meta: &IndexMeta) -> tantivy::Result<()> {
        self.sender
            .send(ChannelRequest::SaveMetas(meta.clone()))
            .unwrap();

        Ok(())
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        self.sender
            .send(ChannelRequest::LoadMetas(inventory.clone()))
            .unwrap();

        match self.receiver.recv().unwrap() {
            ChannelResponse::LoadMetas(metas) => Ok(metas),
            unexpected => Err(tantivy::TantivyError::ErrorInThread(format!(
                "load_metas unexpected response {:?}",
                unexpected
            ))),
        }
    }
}

pub struct ChannelRequestHandler {
    directory: BlockingDirectory,
    relation_oid: pgrx::pg_sys::Oid,
    sender: Sender<ChannelResponse>,
    receiver: Receiver<ChannelRequest>,
    writers: HashMap<PathBuf, SegmentComponentWriter>,
    readers: HashMap<PathBuf, SegmentComponentReader>,
}

impl ChannelRequestHandler {
    pub fn open(
        directory: BlockingDirectory,
        relation_oid: pgrx::pg_sys::Oid,
        sender: Sender<ChannelResponse>,
        receiver: Receiver<ChannelRequest>,
    ) -> Self {
        Self {
            directory,
            relation_oid,
            receiver,
            sender,
            writers: HashMap::new(),
            readers: HashMap::new(),
        }
    }

    pub fn receive_blocking(&mut self, should_delete: Option<impl Fn(u64) -> bool>) -> Result<()> {
        for message in self.receiver.iter() {
            match message {
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
                ChannelRequest::SegmentRead(range, handle) => {
                    let reader =
                        self.readers
                            .entry(handle.path.clone())
                            .or_insert_with(|| unsafe {
                                SegmentComponentReader::new(self.relation_oid, handle)
                            });
                    let data = reader.read_bytes(range)?;
                    self.sender
                        .send(ChannelResponse::Bytes(data.as_slice().to_owned()))?;
                }
                ChannelRequest::SegmentWrite(path, data) => {
                    let writer = self.writers.entry(path.clone()).or_insert_with(|| unsafe {
                        SegmentComponentWriter::new(self.relation_oid, &path)
                    });
                    writer.write_all(&data)?;
                }
                ChannelRequest::SegmentWriteTerminate(path) => {
                    let writer = self.writers.remove(&path).expect("writer should exist");
                    writer.terminate()?;
                }
                ChannelRequest::ShouldDeleteCtids(ctids) => {
                    if let Some(ref should_delete) = should_delete {
                        let filtered_ctids: Vec<u64> = ctids
                            .into_iter()
                            .filter(|&ctid_val| should_delete(ctid_val))
                            .collect();
                        self.sender
                            .send(ChannelResponse::ShouldDeleteCtids(filtered_ctids))?;
                    } else {
                        self.sender
                            .send(ChannelResponse::ShouldDeleteCtids(vec![]))?;
                    }
                }
                ChannelRequest::SaveMetas(metas) => {
                    self.directory.save_metas(&metas)?;
                }
                ChannelRequest::LoadMetas(inventory) => {
                    let metas = self.directory.load_metas(&inventory)?;
                    self.sender.send(ChannelResponse::LoadMetas(metas))?;
                }
                ChannelRequest::Terminate => {
                    break;
                }
            }
        }

        Ok(())
    }
}
