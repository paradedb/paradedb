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

pub fn get_fresh_executor_manager() -> &'static mut Manager {
    // We should call this at the top of a scan to clear out the manager memory.
    // Otherwise, the static manager could grow unbound and leak memory.
    unsafe {
        MANAGER = Manager::new();
        &mut MANAGER
    }
}

type BlockInfo = (BlockNumber, OffsetNumber);

pub struct Manager {
    max_score: f32,
    min_score: f32,
    scores: Option<HashMap<BlockInfo, f32>>,
    doc_addresses: Option<HashMap<BlockInfo, DocAddress>>,
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
        query: &dyn Query,
        highlights_max_num_chars: Option<usize>,
    ) {
        // Because we're adding the whole schema at once, we can replace to make sure
        // that we're adding to a clean hash map.

        self.snippet_generators.replace(HashMap::new());
        for field in schema.fields() {
            let field_name = field.1.name().to_string();

            if let FieldType::Str(_) = field.1.field_type() {
                let mut snippet_generator = SnippetGenerator::create(searcher, query, field.0)
                    .unwrap_or_else(|err| {
                        panic!(
                            "failed to create snippet generator for field: {field_name}... {err}"
                        )
                    });

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

#[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use std::collections::HashMap;

    use pgrx::{
        item_pointer_get_both,
        pg_sys::{BlockIdData, ItemPointerData},
    };
    use tantivy::{
        doc,
        query::{Query, RegexQuery},
        schema::{Field, Schema, TEXT},
        DocAddress, Document, Index, Searcher, SnippetGenerator,
    };

    use crate::manager::BlockInfo;

    use super::{get_current_executor_manager, get_fresh_executor_manager};

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
        let item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 10,
                bi_lo: 0,
            },
            ip_posid: 8,
        };
        let ctid = item_pointer_get_both(item_ptr);

        expected.add_score(ctid, 3.3);
        expected.set_max_score(66.8);
        expected.set_min_score(2.2);

        let manager = get_current_executor_manager();
        assert_eq!(manager.get_min_score(), expected.get_min_score());
        assert_eq!(
            expected.get_max_score() - manager.get_min_score(),
            64.600006
        );
        assert_eq!(manager.get_score(item_ptr), expected.get_score(item_ptr));
    }

    #[pgrx::pg_test]
    fn test_add_score() {
        let first_item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 10,
                bi_lo: 0,
            },
            ip_posid: 8,
        };
        let second_item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 88,
                bi_lo: 22,
            },
            ip_posid: 3,
        };
        let first_ctid = item_pointer_get_both(first_item_ptr);
        let second_ctid = item_pointer_get_both(second_item_ptr);

        let manager = get_fresh_executor_manager();
        manager.add_score(first_ctid, 46.9);
        manager.add_score(second_ctid, 66.5);

        let mut expected: HashMap<BlockInfo, f32> = HashMap::new();
        expected.insert((655360, 8), 46.9);
        expected.insert((5767190, 3), 66.5);

        assert_eq!(expected, manager.scores.clone().unwrap());

        let item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 777,
                bi_lo: 99,
            },
            ip_posid: 333,
        };
        assert_eq!(manager.get_score(item_ptr), None);
    }

    #[pgrx::pg_test]
    fn test_add_doc_address() {
        let first_item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 10,
                bi_lo: 0,
            },
            ip_posid: 8,
        };
        let second_item_ptr = ItemPointerData {
            ip_blkid: BlockIdData {
                bi_hi: 88,
                bi_lo: 22,
            },
            ip_posid: 3,
        };
        let first_ctid = item_pointer_get_both(first_item_ptr);
        let second_ctid = item_pointer_get_both(second_item_ptr);

        let first_doc_address = DocAddress::new(0, 1);
        let second_doc_address = DocAddress::new(0, 2);

        let manager = get_fresh_executor_manager();
        manager.add_doc_address(first_ctid, first_doc_address);
        manager.add_doc_address(second_ctid, second_doc_address);

        let mut expected: HashMap<BlockInfo, DocAddress> = HashMap::new();
        expected.insert((655360, 8), first_doc_address);
        expected.insert((5767190, 3), second_doc_address);

        assert_eq!(&expected, manager.doc_addresses.as_mut().unwrap());
        assert_eq!(manager.doc_addresses.as_mut().unwrap().len(), 2);
    }

    fn prepare_schema() -> tantivy::Result<(Schema, Searcher, Field)> {
        let mut schema_builder = Schema::builder();
        let title = schema_builder.add_text_field("title", TEXT);
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        {
            let mut index_writer = index.writer(3_000_000)?;
            index_writer.add_document(doc!(
                title => "The Name of the Wind",
            ))?;
            index_writer.add_document(doc!(
                title => "The Diary of Muadib",
            ))?;
            index_writer.add_document(doc!(
                title => "A Dairy Cow",
            ))?;
            index_writer.add_document(doc!(
                title => "The Diary of a Young Girl",
            ))?;
            index_writer.commit()?;
        }

        let reader = index.reader()?;
        let searcher = reader.searcher();
        Ok((schema, searcher, title))
    }

    #[pgrx::pg_test]
    fn test_add_snippet_generators() -> tantivy::Result<()> {
        let (schema, searcher, title) = prepare_schema()?;
        let query: Box<dyn Query> = Box::new(RegexQuery::from_pattern("d[ai]{2}ry", title)?);

        let manager = get_fresh_executor_manager();
        manager.add_snippet_generators(&searcher, &schema, &query, Some(3));
        let snippet_generators = manager.snippet_generators.as_mut().unwrap();

        assert_eq!(snippet_generators.len(), 1);
        assert!(snippet_generators.get("title").is_some());
        assert!(snippet_generators.get("id").is_none());

        Ok(())
    }

    #[pgrx::pg_test]
    #[should_panic]
    fn fail_get_highlight() {
        let (schema, searcher, title) = prepare_schema().unwrap();
        let query: Box<dyn Query> =
            Box::new(RegexQuery::from_pattern("d[ai]{2}ry", title).unwrap());

        let manager = get_fresh_executor_manager();
        manager.add_snippet_generators(&searcher, &schema, &query, None);

        let mut doc = Document::default();
        doc.add_text(title, "Diary of The Dairy Cow");

        manager.get_highlight("me", &doc);
    }

    #[pgrx::pg_test]
    fn test_get_highlight() {
        let (schema, searcher, title) = prepare_schema().unwrap();
        let query: Box<dyn Query> =
            Box::new(RegexQuery::from_pattern("d[ai]{2}ry", title).unwrap());

        let manager = get_fresh_executor_manager();
        manager.add_snippet_generators(&searcher, &schema, &query, None);

        let mut doc = Document::default();
        doc.add_text(title, "Diary of The Dairy Cow");

        let highlight = manager.get_highlight("title", &doc);
        assert_eq!(highlight, Some("".to_string()));
    }

    #[pgrx::pg_test]
    fn test_parse_snippet() {
        let (_schema, searcher, title) = prepare_schema().unwrap();
        let query: Box<dyn Query> =
            Box::new(RegexQuery::from_pattern("d[ai]{2}ry", title).unwrap());
        let snippet_generator = SnippetGenerator::create(&searcher, &query, title).unwrap();
        let snippet = snippet_generator.snippet("pg_bm25 is a postgres extension by paradedb");
        assert_eq!("", snippet.to_html());
    }
}
