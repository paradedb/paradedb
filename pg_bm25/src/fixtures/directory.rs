pub use crate::writer::SearchFs;
use crate::writer::{
    ParadeDirectoryError, TantivyDirPath, WriterDirectory, WriterTransferPipeFilePath,
};
use serde::{de::DeserializeOwned, Serialize};

pub struct MockWriterDirectory {
    pub temp_dir: tempfile::TempDir,
    pub writer_dir: WriterDirectory,
}

impl MockWriterDirectory {
    pub fn new(index_name: &str) -> Self {
        // We must store the TempDir instance on the struct, because it gets deleted when the
        // instance is dropped.
        let temp_dir = tempfile::Builder::new()
            .prefix(index_name)
            .tempdir()
            .expect("error creating tempdir for MockWriterDirectory");
        let temp_path = temp_dir.path().to_path_buf();
        Self {
            temp_dir,
            writer_dir: WriterDirectory {
                index_name: index_name.to_string(),
                database_oid: 0,
                postgres_data_dir_path: temp_path,
            },
        }
    }
}

impl SearchFs for MockWriterDirectory {
    fn load_index<T: DeserializeOwned>(&self) -> Result<T, ParadeDirectoryError> {
        self.writer_dir.load_index()
    }
    fn save_index<T: Serialize>(&self, index: &T) -> Result<(), ParadeDirectoryError> {
        self.writer_dir.save_index(index)
    }
    fn remove(&self) -> Result<(), ParadeDirectoryError> {
        self.writer_dir.remove()
    }
    fn tantivy_dir_path(&self) -> Result<TantivyDirPath, ParadeDirectoryError> {
        self.writer_dir.tantivy_dir_path()
    }
    fn writer_transfer_pipe_path(
        &self,
    ) -> Result<WriterTransferPipeFilePath, ParadeDirectoryError> {
        self.writer_dir.writer_transfer_pipe_path()
    }
}
