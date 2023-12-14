use super::index::{ParadeIndex, ParadeIndexKey, ParadeIndexKeyValue};
use crate::{index_access::utils::get_parade_index, json::builder::JsonBuilder};
use pgrx::{
    info, item_pointer_to_u64, pg_sys::ItemPointerData, register_xact_callback, PgXactCallbackEvent,
};
use std::collections::HashMap;
use tantivy::{
    schema::{Field, Value},
    Document, IndexWriter, Searcher, Term,
};

// The
const CACHE_NUM_BLOCKS: usize = 10;

/// This is a transaction-scoped cache. The ParadeWriterCache is a singleton that intializes
/// this cache during index build + insert transactions, and the cache is active for the life of
/// the transaction. The ParadeWriterCache registers a callback to clear this cache at the
/// end of the transaction.
pub static mut PARADE_WRITER_CACHE: ParadeWriterCache = ParadeWriterCache { cache: None };

pub struct ParadeWriter {
    ctid_field: Field,
    key_field: Field,
    fields: HashMap<String, Field>,
    searcher: Searcher,
    writer: IndexWriter,
    pub key_field_name: String,
}

impl ParadeWriter {
    pub fn new(parade_index: &ParadeIndex) -> Self {
        info!("NEW WRITER ON: {}", parade_index.name);
        Self {
            fields: parade_index.fields.clone(),
            ctid_field: parade_index.ctid_field,
            key_field: parade_index.key_field,
            searcher: parade_index.searcher(),
            writer: parade_index.writer().unwrap(),
            key_field_name: parade_index.key_field_name.clone(),
        }
    }

    pub fn from_index_name(index_name: &str) -> Self {
        let parade_index = get_parade_index(index_name);
        Self::new(parade_index)
    }

    pub fn delete_by_key(&self, key: &ParadeIndexKey) {
        // Delete existing index entries with the same key.
        let key_field_term = match key.value {
            ParadeIndexKeyValue::Number(key_value) => {
                Term::from_field_i64(self.key_field, key_value)
            }
        };
        self.writer.delete_term(key_field_term);
    }

    pub fn insert(&mut self, ctid: ItemPointerData, builder: JsonBuilder) {
        let mut doc: Document = Document::new();
        for (col_name, value) in builder.values.iter() {
            let field_option = self.fields.get(col_name.trim_matches('"'));
            if let Some(field) = field_option {
                value.add_to_tantivy_doc(&mut doc, field);
            }
        }

        // Add a ctid field so that we can retrieve the document by ctid in the index scan.
        doc.add_u64(self.ctid_field, item_pointer_to_u64(ctid));
        self.writer
            .add_document(doc)
            .expect("failed to add document");
    }

    pub fn bulk_delete(
        &mut self,
        should_delete_callback: impl Fn(*mut ItemPointerData) -> bool,
    ) -> (u32, u32) {
        let mut deleted: u32 = 0;
        let mut not_deleted: u32 = 0;

        for segment_reader in self.searcher.segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc_id in 0..segment_reader.num_docs() {
                if let Ok(stored_fields) = store_reader.get(doc_id) {
                    if let Some(Value::U64(ctid_val)) = stored_fields.get_first(self.ctid_field) {
                        let mut ctid = ItemPointerData::default();
                        let should_delete = should_delete_callback(&mut ctid);
                        if should_delete {
                            let term_to_delete = Term::from_field_u64(self.ctid_field, *ctid_val);
                            self.writer.delete_term(term_to_delete);
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

    pub fn commit(mut self) {
        self.writer.commit().unwrap();
    }
}

#[derive(Default)]
pub struct ParadeWriterCache {
    cache: Option<HashMap<String, ParadeWriter>>,
}

impl ParadeWriterCache {
    pub fn get_cached(&mut self, index_name: &str) -> &mut ParadeWriter {
        // Initialize the cache if it doesn't exist
        if self.cache.is_none() {
            // If we're here, we are assuming that this is the first invocation for
            // the current transaction, so we'll setup some cleanup functions.

            // The cache is presumably None at this point, so we'll initialize it.
            self.cache = Some(HashMap::new());

            // The index_name argument needs to be cloned to be moved into the closure.
            let index_name_cloned = index_name.to_string();
            let callback = move || unsafe {
                let parade_index = get_parade_index(index_name_cloned.as_str());
                PARADE_WRITER_CACHE
                    .cache
                    .take() // take "clears" the cache by setting it to None
                    .unwrap_or_default()
                    .into_iter()
                    .for_each(|(_, writer)| writer.commit());
                // All the writers must be committed before the reader reloads, or else
                // there will be stale data in the index on the next query.
                parade_index.reader.reload().unwrap();
            };

            // We need to make sure the callback fires both in case of abort and commit,
            // so we'll register identical functions for each event.
            register_xact_callback(PgXactCallbackEvent::Commit, callback.clone());
            register_xact_callback(PgXactCallbackEvent::Abort, callback.clone());
        }

        // Insert the writer if it does not exist.
        self.cache
            .as_mut()
            .unwrap()
            .entry(index_name.to_string())
            .or_insert_with(|| ParadeWriter::from_index_name(index_name))
    }
}
