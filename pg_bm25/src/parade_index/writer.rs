// use super::index::{ParadeIndex, ParadeIndexKey, ParadeIndexKeyValue};
// use crate::{index_access::utils::get_parade_index, json::builder::JsonBuilder};
// use pgrx::{
//     item_pointer_to_u64, pg_sys::ItemPointerData, register_xact_callback, PgXactCallbackEvent,
// };
// use std::{collections::HashMap, fs, io};
// use tantivy::{
//     schema::{Field, Value},
//     Document, IndexWriter, Searcher, Term,
// };

// const CACHE_NUM_BLOCKS: usize = 10;

// /// This is a transaction-scoped cache. The ParadeWriterCache is a singleton that intializes
// /// this cache during index build + insert transactions, and the cache is active for the life of
// /// the transaction. The ParadeWriterCache registers a callback to clear this cache at the
// /// end of the transaction.
// pub static mut PARADE_WRITER_CACHE: ParadeWriterCache = ParadeWriterCache {
//     cache: None,
//     will_clear: false,
// };

// pub struct ParadeWriterOld {
//     ctid_field: Field,
//     key_field: Field,
//     fields: HashMap<String, Field>,
//     searcher: Searcher,
//     writer: IndexWriter,
//     pub lockfile_path: String,
//     pub index_name: String,
//     pub key_field_name: String,
// }

// impl ParadeWriterOld {
//     pub fn new(parade_index: &ParadeIndex) -> Self {
//         Self {
//             fields: parade_index.fields.clone(),
//             ctid_field: parade_index.ctid_field,
//             key_field: parade_index.key_field,
//             searcher: parade_index.searcher(),
//             writer: parade_index.writer().unwrap(),
//             index_name: parade_index.name.clone(),
//             key_field_name: parade_index.key_field_name.clone(),
//             lockfile_path: format!(
//                 "{}/.tantivy-writer.lock",
//                 ParadeIndex::get_data_directory(&parade_index.name)
//             ),
//         }
//     }

//     pub fn from_index_name(index_name: &str) -> Self {
//         let parade_index = get_parade_index(index_name);
//         Self::new(parade_index)
//     }

//     pub fn delete_by_key(&self, key: &ParadeIndexKey) {
//         // Delete existing index entries with the same key.
//         let key_field_term = match key.value {
//             ParadeIndexKeyValue::Number(key_value) => {
//                 Term::from_field_i64(self.key_field, key_value)
//             }
//         };
//         self.writer.delete_term(key_field_term);
//     }

//     // pub fn insert(&mut self, ctid: ItemPointerData, builder: JsonBuilder) {
//     //     let mut doc: Document = Document::new();
//     //     for (col_name, value) in builder.values.iter() {
//     //         let field_option = self.fields.get(col_name.trim_matches('"'));
//     //         if let Some(field) = field_option {
//     //             value.add_to_tantivy_doc(&mut doc, field);
//     //         }
//     //     }

//     //     // Add a ctid field so that we can retrieve the document by ctid in the index scan.
//     //     doc.add_u64(self.ctid_field, item_pointer_to_u64(ctid));
//     //     self.writer
//     //         .add_document(doc)
//     //         .expect("failed to add document");
//     // }

//     pub fn bulk_delete(
//         &mut self,
//         should_delete_callback: impl Fn(*mut ItemPointerData) -> bool,
//     ) -> (u32, u32) {
//         let mut deleted: u32 = 0;
//         let mut not_deleted: u32 = 0;

//         for segment_reader in self.searcher.segment_readers() {
//             let store_reader = segment_reader
//                 .get_store_reader(CACHE_NUM_BLOCKS)
//                 .expect("Failed to get store reader");

//             for doc_id in 0..segment_reader.num_docs() {
//                 if let Ok(stored_fields) = store_reader.get(doc_id) {
//                     if let Some(Value::U64(ctid_val)) = stored_fields.get_first(self.ctid_field) {
//                         let mut ctid = ItemPointerData::default();
//                         let should_delete = should_delete_callback(&mut ctid);
//                         if should_delete {
//                             let term_to_delete = Term::from_field_u64(self.ctid_field, *ctid_val);
//                             self.writer.delete_term(term_to_delete);
//                             deleted += 1;
//                         } else {
//                             not_deleted += 1;
//                         }
//                     }
//                 }
//             }
//         }

//         (deleted, not_deleted)
//     }

//     pub fn commit(mut self) {
//         self.writer.commit().unwrap();
//     }

//     pub fn garbage_collect(&self) {
//         self.writer
//             .garbage_collect_files()
//             .wait()
//             .expect("Could not collect garbage");
//     }
// }

// #[derive(Default)]
// pub struct ParadeWriterCache {
//     cache: Option<HashMap<String, ParadeWriterOld>>,
//     will_clear: bool,
// }

// impl ParadeWriterCache {
//     /// Get a cached ParadeWriter, or acquire one and cache it.
//     pub fn get_cached(&mut self, index_name: &str) -> &mut ParadeWriterOld {
//         // Initialize the cache if it doesn't exist
//         if self.cache.is_none() {
//             // If we're here, this is the first invocation for the transaction.
//             // The cache is presumably None at this point, so we'll initialize it.
//             self.cache = Some(HashMap::new());
//         }

//         // Insert the writer if it does not exist.
//         self.cache
//             .as_mut()
//             .unwrap()
//             .entry(index_name.to_string())
//             .or_insert_with(|| ParadeWriterOld::from_index_name(index_name))
//     }

//     /// Internal implementation of cache clearing.
//     fn clear() {
//         unsafe {
//             for (_, writer) in PARADE_WRITER_CACHE
//                 .cache
//                 .take() // take "clears" the cache by setting it to None
//                 .unwrap_or_default()
//             {
//                 let lockfile_path = writer.lockfile_path.clone();
//                 let parade_index = get_parade_index(&writer.index_name);
//                 writer.commit();

//                 // The must be committed before the corresponding reader reloads, or else
//                 // there will be stale data in the index on the next query.
//                 parade_index.reader.reload().unwrap();

//                 // Make sure the lockfile on the writer is deleted. This should be done automatically
//                 // after the Tantivy writers are dropped, but in practice the file can stick around.
//                 match fs::remove_file(&lockfile_path) {
//                     Ok(()) => Ok(()),
//                     Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
//                     Err(e) => Err(e),
//                 }
//                 .expect("could not remove tantivy lockfile");
//             }

//             // Make sure the will_clear flag is set back to false, so callbacks can be
//             // registered on the next transaction.
//             PARADE_WRITER_CACHE.will_clear = false;

//             // It's important that the cache be reset to None, so the ParadeWriter
//             // instances that it holds are dropped. If this does not happen, the locks
//             // held by ParadeWriter will not be released.
//             PARADE_WRITER_CACHE.cache = None;
//         }
//     }

//     /// Manually clear the ParadeWriterCache.
//     pub fn clear_cache(&self) {
//         // We have this calling out to an internal static method, because in a separate
//         // method where we register callbacks, the callbacks cannot contain references to
//         // `self`. This method is for convenience, so it can be called off the singleton
//         // PARADE_WRITER_CACHE instance.
//         Self::clear();
//     }

//     /// Register callbacks to clear the ParadeWriterCache on transaction end.
//     /// This must be manually called, because some index access methods like ambuild
//     /// and amvacuumcleanup need to be able to clear the cache manually, without waiting
//     /// for the end of the transaction.
//     pub fn clear_cache_on_transaction_end(&mut self) {
//         // We check this flag to make sure this function is idempotent.
//         // A caller like aminsert can safely call this over and over.
//         if !self.will_clear {
//             // We need to make sure the callback fires both in case of abort and commit,
//             // so we'll register identical functions for each event.
//             register_xact_callback(PgXactCallbackEvent::Commit, Self::clear);
//             register_xact_callback(PgXactCallbackEvent::Abort, Self::clear);
//             self.will_clear = true;
//         }
//     }
// }
