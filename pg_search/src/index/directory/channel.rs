use crate::index::mvcc::MVCCDirectory;
use crate::index::reader::channel::ChannelReader;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::channel::ChannelWriter;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::storage::block::{bm25_max_free_space, FileEntry};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::any::Any;
use std::collections::HashSet;
use std::fmt::Debug;
use std::panic::panic_any;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{io, io::Write, ops::Range, result};
use tantivy::directory::error::{DeleteError, LockError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    DirectoryLock, DirectoryPanicHandler, FileHandle, Lock, OwnedBytes, TerminatingWrite,
    WatchCallback, WatchHandle, WritePtr,
};
use tantivy::index::SegmentMetaInventory;
use tantivy::{Directory, IndexMeta, TantivyError};

pub type Overwrite = bool;

#[derive(Debug)]
pub enum ChannelRequest {
    RegisterFilesAsManaged(
        Vec<PathBuf>,
        Overwrite,
        oneshot::Sender<tantivy::Result<()>>,
    ),
    SegmentRead(
        Range<usize>,
        FileEntry,
        oneshot::Sender<std::io::Result<OwnedBytes>>,
    ),
    SegmentWrite(PathBuf, Vec<u8>, oneshot::Sender<std::io::Result<()>>),
    SegmentFlush(PathBuf, oneshot::Sender<std::io::Result<()>>),
    SegmentWriteTerminate(PathBuf, oneshot::Sender<std::io::Result<()>>),
    GetSegmentComponent(PathBuf, oneshot::Sender<tantivy::Result<FileEntry>>),
    SaveMetas(IndexMeta, IndexMeta, oneshot::Sender<tantivy::Result<()>>),
    LoadMetas(
        SegmentMetaInventory,
        oneshot::Sender<tantivy::Result<IndexMeta>>,
    ),
    Panic(Box<dyn Any + Send>),
    WantsCancel(oneshot::Sender<bool>),
    Log(String),
}
#[derive(Clone, Debug)]
pub struct ChannelDirectory {
    sender: Sender<ChannelRequest>,
}

// A directory that actually forwards all read/write requests to a channel
// This channel is used to communicate with the actual storage implementation
impl ChannelDirectory {
    pub fn new(sender: Sender<ChannelRequest>) -> Self {
        Self { sender }
    }
}

impl Directory for ChannelDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        Ok(Arc::new(unsafe {
            ChannelReader::new(path, self.sender.clone()).map_err(|e| {
                OpenReadError::wrap_io_error(
                    io::Error::new(io::ErrorKind::NotConnected, e),
                    path.to_path_buf(),
                )
            })?
        }))
    }

    fn open_write(&self, path: &Path) -> result::Result<WritePtr, OpenWriteError> {
        Ok(io::BufWriter::with_capacity(
            bm25_max_free_space(),
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
        // because we don't support garbage collection
        unimplemented!("list_managed_files should not be called");
    }

    fn register_files_as_managed(
        &self,
        files: Vec<PathBuf>,
        overwrite: bool,
    ) -> tantivy::Result<()> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        self.sender
            .send(ChannelRequest::RegisterFilesAsManaged(
                files,
                overwrite,
                oneshot_sender,
            ))
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e.to_string()))?;

        oneshot_receiver
            .recv()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e))?
    }

    fn save_metas(
        &self,
        meta: &IndexMeta,
        previous_meta: &IndexMeta,
        _payload: &mut (dyn Any + '_),
    ) -> tantivy::Result<()> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        self.sender
            .send(ChannelRequest::SaveMetas(
                meta.clone(),
                previous_meta.clone(),
                oneshot_sender,
            ))
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e.to_string()))?;

        oneshot_receiver
            .recv()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e))?
    }

    fn load_metas(&self, inventory: &SegmentMetaInventory) -> tantivy::Result<IndexMeta> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        self.sender
            .send(ChannelRequest::LoadMetas(inventory.clone(), oneshot_sender))
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e.to_string()))?;

        oneshot_receiver
            .recv()
            .map_err(|e| io::Error::new(io::ErrorKind::NotConnected, e))?
    }

    fn supports_garbage_collection(&self) -> bool {
        false
    }

    fn panic_handler(&self) -> Option<DirectoryPanicHandler> {
        let sender = self.sender.clone();
        let panic_handler = move |any| {
            eprintln!("panic handler got one: {any:?}");
            sender.send(ChannelRequest::Panic(any)).ok();
        };
        Some(Arc::new(panic_handler))
    }

    fn wants_cancel(&self) -> bool {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        if self
            .sender
            .send(ChannelRequest::WantsCancel(oneshot_sender))
            .is_err()
        {
            // we definitely need to cancel if we had a problem
            // sending the message over our internal channel
            return true;
        }

        // similarly, if we had a failure receiving the error we need to go ahead and cancel too
        oneshot_receiver.recv().unwrap_or(true)
    }

    fn log(&self, message: &str) {
        self.sender
            .send(ChannelRequest::Log(message.to_string()))
            .ok(); // silently ignore errors trying to log
    }
}

type Action<'a> = Box<dyn FnOnce() -> Reply + Send + Sync + 'a>;
type Reply = Box<dyn Any + Send + Sync>;
pub struct ChannelRequestHandler {
    directory: MVCCDirectory,
    relation_oid: pg_sys::Oid,
    receiver: Receiver<ChannelRequest>,
    writers: FxHashMap<PathBuf, SegmentComponentWriter>,
    readers: FxHashMap<FileEntry, SegmentComponentReader>,

    file_entries: FxHashMap<PathBuf, FileEntry>,

    action: (Sender<Action<'static>>, Receiver<Action<'static>>),
    reply: (Sender<Reply>, Receiver<Reply>),
    _worker: JoinHandle<()>,
}

impl ChannelRequestHandler {
    pub fn open(
        directory: MVCCDirectory,
        relation_oid: pg_sys::Oid,
        receiver: Receiver<ChannelRequest>,
    ) -> Self {
        let (action_sender, action_receiver) = crossbeam::channel::bounded(1);
        let (reply_sender, reply_receiver) = crossbeam::channel::bounded(1);
        Self {
            directory,
            relation_oid,
            receiver,
            writers: Default::default(),
            readers: Default::default(),
            file_entries: Default::default(),
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

    #[track_caller]
    pub fn wait_for<'me, T: Send + Sync + 'static, F: FnOnce() -> T + Send + Sync + 'me>(
        &'me mut self,
        func: F,
    ) -> Result<T> {
        self.wait_for_internal(func, false)
    }

    #[track_caller]
    pub fn wait_for_final<T: Send + Sync + 'static, F: FnOnce() -> T + Send + Sync>(
        mut self,
        func: F,
    ) -> Result<T> {
        self.wait_for_internal(func, true)
    }

    #[track_caller]
    fn wait_for_internal<'me, T: Send + Sync + 'static, F: FnOnce() -> T + Send + Sync + 'me>(
        &'me mut self,
        func: F,
        sync: bool,
    ) -> Result<T> {
        // Before we fire off the caller's action we should ensure there are no unprocessed messages
        let receiver = self.receiver.clone();
        for message in receiver.try_iter() {
            self.process_message(message)?;
        }

        let boxed_func: Action<'static> = unsafe {
            let boxed_func: Action<'me> = Box::new(move || Box::new(func()));

            // SAFETY
            //
            // What we're doing here is transmuting the lifetime of the `FnOnce() -> T` argument
            // `func` from `'me` (meaning it's assumed to borrow from `'self`) to`'static`.
            //
            // This is safe because despite the closure getting passed to a background
            // thread, we actually wait on it through the internal `self.action` and `self.reply` channels.
            std::mem::transmute(boxed_func)
        };

        self.action.0.send(boxed_func)?;
        loop {
            match self.reply.1.try_recv() {
                // `func` has finished and we have its reply
                Ok(reply) => {
                    return match reply.downcast::<T>() {
                        // the reply is exactly what we hoped for
                        Ok(reply) => {
                            if sync {
                                // in sync mode we need to ensure we've waited for all possible messages
                                // before we return control back to the caller, which will be dropping
                                // this channel
                                let receiver = self.receiver.clone();
                                for message in receiver {
                                    pgrx::debug1!("finalization message={message:?}");
                                    self.process_message(message)?;
                                }
                            }
                            Ok(*reply)
                        }

                        // it's something else, so transform into a generic error
                        Err(e) => Err(anyhow::anyhow!("unexpected reply {:?}", e)),
                    };
                }

                // we have no reply yet, so process any messages it may have generated
                Err(TryRecvError::Empty) => {
                    let receiver = self.receiver.clone();
                    for message in receiver.try_iter() {
                        self.process_message(message)?;
                    }
                }

                // the reply channel was closed, so lets just return that as the error
                Err(TryRecvError::Disconnected) => {
                    return Err(anyhow::anyhow!("reply channel disconnected"));
                }
            }
        }
    }

    fn process_message(&mut self, message: ChannelRequest) -> Result<()> {
        match message {
            ChannelRequest::RegisterFilesAsManaged(files, overwrite, sender) => {
                sender.send(self.directory.register_files_as_managed(files, overwrite))?;
            }
            ChannelRequest::GetSegmentComponent(path, sender) => {
                if self.file_entries.contains_key(&path) {
                    sender.send(Ok(*self.file_entries.get(&path).unwrap()))?;
                } else {
                    let file_entry = unsafe {
                        self.directory
                            .directory_lookup(&path)
                            .map_err(|e| TantivyError::SystemError(e.to_string()))
                    };
                    sender.send(file_entry)?;
                }
            }
            ChannelRequest::SegmentRead(range, handle, sender) => {
                let reader = self.readers.entry(handle).or_insert_with(|| unsafe {
                    SegmentComponentReader::new(self.relation_oid, handle)
                });
                sender.send(reader.read_bytes(range))?;
            }
            ChannelRequest::SegmentWrite(path, data, sender) => {
                let writer = self.writers.entry(path.clone()).or_insert_with(|| unsafe {
                    SegmentComponentWriter::new(self.relation_oid, &path)
                });
                sender.send(writer.write_all(&data))?
            }
            ChannelRequest::SegmentFlush(path, sender) => {
                if let Some(writer) = self.writers.get_mut(&path) {
                    sender.send(writer.flush())?
                } else {
                    sender.send(Ok(()))?
                }
            }
            ChannelRequest::SegmentWriteTerminate(path, sender) => {
                let writer = self.writers.remove(&path).expect("writer should exist");
                self.file_entries.insert(writer.path(), writer.file_entry());
                sender.send(writer.terminate())?;
            }
            ChannelRequest::SaveMetas(metas, previous_metas, sender) => {
                let result =
                    self.directory
                        .save_metas(&metas, &previous_metas, &mut self.file_entries);
                sender.send(result)?;
            }
            ChannelRequest::LoadMetas(inventory, sender) => {
                sender.send(self.directory.load_metas(&inventory))?;
            }
            ChannelRequest::Panic(any) => {
                if let Some(panic_handler) = self.directory.panic_handler() {
                    panic_handler(any);
                } else {
                    panic_any(any)
                }
            }
            ChannelRequest::WantsCancel(sender) => {
                sender.send(self.directory.wants_cancel())?;
            }
            ChannelRequest::Log(message) => self.directory.log(&message),
        }
        Ok(())
    }
}
