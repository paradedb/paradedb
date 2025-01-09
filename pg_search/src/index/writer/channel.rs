use crate::index::directory::channel::ChannelRequest;
use crossbeam::channel::Sender;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};
use tantivy::directory::{AntiCallToken, TerminatingWrite};

pub struct ChannelWriter {
    path: PathBuf,
    sender: Sender<ChannelRequest>,
}

impl ChannelWriter {
    pub unsafe fn new(path: &Path, sender: Sender<ChannelRequest>) -> Self {
        Self {
            path: path.to_path_buf(),
            sender,
        }
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.sender
            .send(ChannelRequest::SegmentWrite(
                self.path.clone(),
                data.to_vec(),
            ))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))?;
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.sender
            .send(ChannelRequest::SegmentFlush(self.path.clone()))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))?;
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        let _ = self.write(buf)?;
        Ok(())
    }
}

impl TerminatingWrite for ChannelWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<()> {
        self.sender
            .send(ChannelRequest::SegmentWriteTerminate(self.path.clone()))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{e}")))
    }
}
