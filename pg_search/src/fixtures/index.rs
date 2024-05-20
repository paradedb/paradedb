use crate::{
    index::SearchIndex,
    schema::{SearchFieldConfig, SearchFieldName, SearchFieldType},
};

use super::MockWriterDirectory;

pub struct MockSearchIndex {
    pub directory: MockWriterDirectory,
    pub index: &'static mut SearchIndex,
}

impl MockSearchIndex {
    pub fn new(fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>) -> Self {
        // We must store the TempDir instance on the struct, because it gets deleted when the
        // instance is dropped.
        let directory = MockWriterDirectory::new("mock_parade_search_index");
        let index = SearchIndex::new(directory.writer_dir.clone(), fields).unwrap();
        Self { directory, index }
    }
}
