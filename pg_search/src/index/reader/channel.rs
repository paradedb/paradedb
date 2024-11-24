use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use std::ops::Range;
use std::path::Path;
use tantivy::directory::FileHandle;
use tantivy::directory::OwnedBytes;
use tantivy::HasLen;

use crate::index::directory::channel::{ChannelRequest, ChannelResponse};
use crate::postgres::storage::block::DirectoryEntry;

#[derive(Clone, Debug)]
pub struct ChannelReader {
    opaque: DirectoryEntry,
    sender: Sender<ChannelRequest>,
    receiver: Receiver<ChannelResponse>,
}

impl ChannelReader {
    pub unsafe fn new(
        path: &Path,
        sender: Sender<ChannelRequest>,
        receiver: Receiver<ChannelResponse>,
    ) -> Result<Self> {
        sender
            .send(ChannelRequest::GetSegmentComponent(path.to_path_buf()))
            .unwrap();
        let opaque = match receiver.recv().unwrap() {
            ChannelResponse::DirectoryEntry(opaque) => opaque,
            unexpected => panic!("DirectoryEntry expected, got {:?}", unexpected),
        };

        Ok(Self {
            opaque,
            sender,
            receiver,
        })
    }
}

impl FileHandle for ChannelReader {
    fn read_bytes(&self, range: Range<usize>) -> Result<OwnedBytes, std::io::Error> {
        self.sender
            .send(ChannelRequest::SegmentRead(
                range.clone(),
                self.opaque.clone(),
            ))
            .unwrap();
        let data = match self.receiver.recv().unwrap() {
            ChannelResponse::Bytes(data) => data,
            unexpected => panic!("Bytes expected, got {:?}", unexpected),
        };

        Ok(OwnedBytes::new(data))
    }
}

impl HasLen for ChannelReader {
    fn len(&self) -> usize {
        self.opaque.total_bytes
    }
}
