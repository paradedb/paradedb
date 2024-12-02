use anyhow::Result;
use crossbeam::channel::Sender;
use std::ops::Range;
use std::path::Path;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::index::directory::channel::ChannelRequest;
use crate::postgres::storage::block::DirectoryEntry;

#[derive(Clone, Debug)]
pub struct ChannelReader {
    opaque: DirectoryEntry,
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
            .unwrap();

        let opaque = oneshot_receiver.recv()?;
        Ok(Self { opaque, sender })
    }
}

impl FileHandle for ChannelReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, std::io::Error> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        self.sender
            .send(ChannelRequest::SegmentRead(
                range.clone(),
                self.opaque.clone(),
                oneshot_sender,
            ))
            .unwrap();

        let data = oneshot_receiver.recv().unwrap();
        Ok(OwnedBytes::new(data))
    }
}

impl HasLen for ChannelReader {
    fn len(&self) -> usize {
        self.opaque.total_bytes
    }
}
