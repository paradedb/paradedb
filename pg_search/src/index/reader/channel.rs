use anyhow::Result;
use crossbeam::channel::Sender;
use std::ops::Range;
use std::path::Path;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::index::directory::channel::ChannelRequest;
use crate::postgres::storage::block::FileEntry;

#[derive(Clone, Debug)]
pub struct ChannelReader {
    entry: FileEntry,
    sender: Sender<ChannelRequest>,
}

impl ChannelReader {
    pub unsafe fn new(path: &Path, sender: Sender<ChannelRequest>) -> Result<Self> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        sender
            .send(ChannelRequest::GetSegmentComponent(
                path.to_path_buf(),
                oneshot_sender,
            ))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotConnected, format!("{e:?}")))?;

        let entry = oneshot_receiver.recv()?;
        Ok(Self { entry, sender })
    }
}

impl FileHandle for ChannelReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, std::io::Error> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        self.sender
            .send(ChannelRequest::SegmentRead(
                range.clone(),
                self.entry,
                oneshot_sender,
            ))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string()))?;

        oneshot_receiver
            .recv()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotConnected, e))
    }
}

impl HasLen for ChannelReader {
    fn len(&self) -> usize {
        self.entry.total_bytes
    }
}
