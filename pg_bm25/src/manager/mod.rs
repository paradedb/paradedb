use std::collections::HashMap;
use tantivy::{DocAddress, Document, Snippet, SnippetGenerator};

static mut MANAGER: Manager = Manager::new();

pub fn get_current_executor_manager() -> &'static mut Manager {
    unsafe { &mut MANAGER }
}

pub fn get_fresh_executor_manager() -> &'static mut Manager {
    // We should call this at the top of a scan to clear out the manager memory.
    // Otherwise, the static manager could grow unbound and leak memory.
    unsafe {
        MANAGER = Manager::new();
        &mut MANAGER
    }
}

pub struct Manager {
    max_score: f32,
    min_score: f32,
    scores: Option<HashMap<i64, f32>>,
    doc_addresses: Option<HashMap<i64, DocAddress>>,
    snippet_generators: Option<HashMap<String, SnippetGenerator>>,
}

impl Manager {
    pub const fn new() -> Self {
        Self {
            scores: None,
            max_score: 0.0,
            min_score: 0.0,
            doc_addresses: None,
            snippet_generators: None,
        }
    }

    pub fn add_score(&mut self, bm25_id: i64, score: f32) {
        if self.scores.is_none() {
            self.scores.replace(HashMap::new());
        }

        self.scores.as_mut().unwrap().insert(bm25_id, score);
    }

    pub fn get_score(&mut self, bm25_id: i64) -> Option<f32> {
        self.scores.as_mut().unwrap().get(&bm25_id).copied()
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

    pub fn add_doc_address(&mut self, bm25_id: i64, doc_address: DocAddress) {
        if self.doc_addresses.is_none() {
            self.doc_addresses.replace(HashMap::new());
        }

        self.doc_addresses
            .as_mut()
            .unwrap()
            .insert(bm25_id, doc_address);
    }

    pub fn get_doc_address(&mut self, bm25_id: i64) -> Option<DocAddress> {
        self.doc_addresses.as_mut().unwrap().get(&bm25_id).copied()
    }

    pub fn set_snippet_generators(
        &mut self,
        snippet_generators: HashMap<String, SnippetGenerator>,
    ) {
        self.snippet_generators.replace(snippet_generators);
    }

    pub fn get_highlight(&mut self, field_name: &str, doc: &Document) -> Option<String> {
        let snippet_generator_map = self
            .snippet_generators
            .as_ref()
            .expect("snippet generators not correctly initialized");

        let snippet_generator = snippet_generator_map.get(field_name).unwrap_or_else(|| {
            panic!("failed to retrieve snippet generator to highlight field: {field_name}...")
        });

        let snippet = snippet_generator.snippet_from_doc(doc);

        Some(self.parse_snippet(snippet))
    }

    fn parse_snippet(&self, snippet: Snippet) -> String {
        snippet.to_html()
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::{get_current_executor_manager, get_fresh_executor_manager};
    use std::collections::HashMap;
    use tantivy::doc;
    use tantivy::DocAddress;

    #[pgrx::pg_test]
    fn test_fresh_executor_manager() {
        let manager = get_fresh_executor_manager();
        assert_eq!(manager.scores, None);
        assert_eq!(manager.max_score, 0.0);
        assert_eq!(manager.min_score, 0.0);
    }

    #[pgrx::pg_test]
    fn test_current_executor_manager() {
        let expected = get_fresh_executor_manager();
        let key = 25;

        expected.add_score(key, 3.3);
        expected.set_max_score(66.8);
        expected.set_min_score(2.2);

        let manager = get_current_executor_manager();
        assert_eq!(manager.get_min_score(), expected.get_min_score());
        assert_eq!(
            expected.get_max_score() - manager.get_min_score(),
            64.600006
        );
        assert_eq!(manager.get_score(key), expected.get_score(key));
    }

    #[pgrx::pg_test]
    fn test_add_score() {
        let first_key = 25;
        let second_key = 35;

        let manager = get_fresh_executor_manager();
        manager.add_score(first_key, 46.9);
        manager.add_score(second_key, 66.5);

        let mut expected: HashMap<i64, f32> = HashMap::new();
        expected.insert(first_key, 46.9);
        expected.insert(second_key, 66.5);

        assert_eq!(expected, manager.scores.clone().unwrap());

        let third_key = 45;
        assert_eq!(manager.get_score(third_key), None);
    }

    #[pgrx::pg_test]
    fn test_add_doc_address() {
        let first_key = 55;
        let second_key = 65;

        let first_doc_address = DocAddress::new(0, 1);
        let second_doc_address = DocAddress::new(0, 2);

        let manager = get_fresh_executor_manager();
        manager.add_doc_address(first_key, first_doc_address);
        manager.add_doc_address(second_key, second_doc_address);

        let mut expected: HashMap<i64, DocAddress> = HashMap::new();
        expected.insert(first_key, first_doc_address);
        expected.insert(second_key, second_doc_address);

        assert_eq!(&expected, manager.doc_addresses.as_mut().unwrap());
        assert_eq!(manager.doc_addresses.as_mut().unwrap().len(), 2);
    }
}
