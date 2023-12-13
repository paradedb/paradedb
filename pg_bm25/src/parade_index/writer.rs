// use std::fs;

use pgrx::{item_pointer_to_u64, pg_sys::ItemPointerData};
use tantivy::{schema::Value, Document, IndexWriter, Term};

use crate::json::builder::JsonBuilder;

use super::index::ParadeIndex;

const CACHE_NUM_BLOCKS: usize = 10;
pub struct ParadeWriter<'a> {
    parade_index: &'a ParadeIndex,
    underlying_writer: IndexWriter,
}

impl<'a> ParadeWriter<'a> {
    pub fn new(parade_index: &'a ParadeIndex) -> Self {
        Self {
            parade_index,
            underlying_writer: parade_index.writer().unwrap(),
        }
    }

    // fn get_lockfile_path(&self) -> String {
    //     let dir_path = ParadeIndex::get_data_directory(&self.parade_index.name);
    //     format!("{dir_path}/.tantivy-writer.lock")
    // }

    pub fn insert(&mut self, ctid: ItemPointerData, builder: JsonBuilder) {
        // This method is both an implemenation for `self.insert`, and used publicly
        // during index build, where we want to make sure that the same writer is used
        // for the entire build.
        let mut doc: Document = Document::new();
        for (col_name, value) in builder.values.iter() {
            let field_option = self.parade_index.fields.get(col_name.trim_matches('"'));
            if let Some(field) = field_option {
                value.add_to_tantivy_doc(&mut doc, field);
            }
        }

        let field_option = self.parade_index.fields.get("ctid");
        doc.add_u64(*field_option.unwrap(), item_pointer_to_u64(ctid));
        self.underlying_writer
            .add_document(doc)
            .expect("failed to add document");
    }

    pub fn bulk_delete(
        &mut self,
        should_delete_callback: impl Fn(*mut ItemPointerData) -> bool,
    ) -> (u32, u32) {
        let mut deleted: u32 = 0;
        let mut not_deleted: u32 = 0;

        let ctid_field = self
            .parade_index
            .schema()
            .get_field("ctid")
            .expect("Field 'ctid' not found in schema");

        let searcher = self.parade_index.searcher();

        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc_id in 0..segment_reader.num_docs() {
                if let Ok(stored_fields) = store_reader.get(doc_id) {
                    if let Some(Value::U64(ctid_val)) = stored_fields.get_first(ctid_field) {
                        let mut ctid = ItemPointerData::default();
                        let should_delete = should_delete_callback(&mut ctid);
                        if should_delete {
                            let term_to_delete = Term::from_field_u64(ctid_field, *ctid_val);
                            self.underlying_writer.delete_term(term_to_delete);
                            deleted += 1;
                        } else {
                            not_deleted += 1;
                        }
                    }
                }
            }
        }

        (deleted, not_deleted)
    }
}

impl<'a> Drop for ParadeWriter<'a> {
    fn drop(&mut self) {
        self.underlying_writer.prepare_commit().unwrap();
        self.underlying_writer.commit().unwrap();
        self.parade_index.reader.reload().unwrap();
        // fs::remove_file(self.get_lockfile_path()).unwrap();
    }
}
