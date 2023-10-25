use pgrx::{
    item_pointer_get_both,
    pg_sys::{BlockNumber, ItemPointerData, OffsetNumber},
};
use std::collections::HashMap;
use tantivy::{
    query::Query,
    schema::{FieldType, Schema},
    DocAddress, Document, Searcher, Snippet, SnippetGenerator,
};

static mut MANAGER: Manager = Manager::new();

pub fn get_current_executor_manager() -> &'static mut Manager {
    unsafe { &mut MANAGER }
}

// pub fn get_fresh_executor_manager() -> &'static mut Manager {
//     // We should call this at the top of a scan to clear out the manager memory.
//     // Otherwise, the static manager could grow unbound and leak memory.
//     unsafe {
//         MANAGER = Manager::new();
//         &mut MANAGER
//     }
// }

type BlockInfo = (BlockNumber, OffsetNumber);

pub struct Manager {
    max_score: f32,
    min_score: f32,
    scores: Option<HashMap<BlockInfo, f32>>,
    doc_addresses: Option<HashMap<BlockInfo, DocAddress>>,
    snippet_generators: Option<HashMap<String, SnippetGenerator>>,
    // highlights_max_num_chars: Option<usize>,
}

impl Manager {
    pub const fn new() -> Self {
        Self {
            scores: None,
            max_score: 0.0,
            min_score: 0.0,
            doc_addresses: None,
            snippet_generators: None,
            // highlights_max_num_chars: None,
        }
    }

    pub fn add_score(&mut self, ctid: (BlockNumber, OffsetNumber), score: f32) {
        if self.scores.is_none() {
            self.scores.replace(HashMap::new());
        }

        self.scores.as_mut().unwrap().insert(ctid, score);
    }

    pub fn get_score(&mut self, ctid: ItemPointerData) -> Option<f32> {
        let (block, offset) = item_pointer_get_both(ctid);
        self.scores.as_mut().unwrap().get(&(block, offset)).copied()
    }

    pub fn set_max_score(&mut self, max_score: f32) {
        self.max_score = max_score;
    }

    pub fn get_max_score(&self) -> f32 {
        self.max_score
    }

    pub fn set_min_score(&mut self, min_score: f32) {
        self.min_score = min_score;
    }

    pub fn get_min_score(&self) -> f32 {
        self.min_score
    }

    // pub fn set_highlight_max_num_chars(&mut self, max_num_chars: usize) {
    //     self.highlights_max_num_chars = max_num_chars.into();
    // }

    // pub fn get_highlight_max_num_chars(&self) -> Option<usize> {
    //     self.highlights_max_num_chars
    // }

    pub fn add_doc_address(&mut self, ctid: (BlockNumber, OffsetNumber), doc_address: DocAddress) {
        if self.doc_addresses.is_none() {
            self.doc_addresses.replace(HashMap::new());
        }

        self.doc_addresses
            .as_mut()
            .unwrap()
            .insert(ctid, doc_address);
    }

    pub fn get_doc_address(&mut self, ctid: ItemPointerData) -> Option<DocAddress> {
        let (block, offset) = item_pointer_get_both(ctid);
        self.doc_addresses
            .as_mut()
            .unwrap()
            .get(&(block, offset))
            .copied()
    }

    pub fn add_snippet_generators(
        &mut self,
        searcher: &Searcher,
        schema: &Schema,
        query: &Box<dyn Query>,
        highlights_max_num_chars: Option<usize>,
    ) {
        // Because we're adding the whole schema at once, we can replace to make sure
        // that we're adding to a clean hash map.

        self.snippet_generators.replace(HashMap::new());
        for field in schema.fields() {
            let field_name = field.1.name().to_string();

            if let FieldType::Str(_) = field.1.field_type() {
                let mut snippet_generator = SnippetGenerator::create(searcher, query, field.0)
                    .expect("failed to create snippet generator for field: {field_name}");

                if let Some(max_num_chars) = highlights_max_num_chars {
                    snippet_generator.set_max_num_chars(max_num_chars);
                }

                self.snippet_generators
                    .as_mut()
                    .unwrap()
                    .insert(field_name, snippet_generator);
            }
        }
    }

    pub fn get_highlight(&mut self, field_name: &str, doc: &Document) -> Option<String> {
        let snippet_generator_map = self
            .snippet_generators
            .as_ref()
            .expect("snippet generators not correctly initialized");

        let snippet_generator = snippet_generator_map
            .get(field_name)
            .expect("failed to retrieve snippet generator for field: {field_name}");

        let snippet = snippet_generator.snippet_from_doc(doc);

        Some(self.parse_snippet(snippet))
    }

    fn parse_snippet(&self, snippet: Snippet) -> String {
        snippet.to_html()
    }
}
