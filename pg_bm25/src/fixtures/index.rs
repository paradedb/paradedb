use crate::{
    parade_index::index::ParadeIndex,
    schema::{SearchFieldConfig, SearchFieldName},
};

use super::MockWriterDirectory;

pub struct MockParadeIndex {
    pub directory: MockWriterDirectory,
    pub index: &'static mut ParadeIndex,
}

impl MockParadeIndex {
    pub fn new(fields: Vec<(SearchFieldName, SearchFieldConfig)>) -> Self {
        // We must store the TempDir instance on the struct, because it gets deleted when the
        // instance is dropped.
        let directory = MockWriterDirectory::new("mock_parade_index");
        let index = ParadeIndex::new(directory.writer_dir.clone(), fields).unwrap();
        Self { directory, index }
    }
}
