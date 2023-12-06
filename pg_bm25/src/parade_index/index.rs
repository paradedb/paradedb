use pgrx::pg_sys::{IndexBulkDeleteCallback, IndexBulkDeleteResult, ItemPointerData};
use pgrx::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use shared::plog;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fs::{self, create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::Path;
use tantivy::{query::QueryParser, schema::*, Document, Index, IndexSettings, Searcher, Term};
use tantivy::{IndexReader, IndexWriter, SingleSegmentIndexWriter, TantivyError};

use crate::index_access::options::ParadeOptions;
use crate::index_access::utils::SearchConfig;
use crate::json::builder::{JsonBuilder, JsonBuilderValue};
use crate::parade_index::fields::{ParadeOption, ParadeOptionMap};
use crate::tokenizers::{create_normalizer_manager, create_tokenizer_manager};

use super::state::TantivyScanState;

const CACHE_NUM_BLOCKS: usize = 10;
const INDEX_TANTIVY_MEMORY_BUDGET: usize = 50_000_000;

/// PostgreSQL operates in a process-per-client model, meaning every client connection
/// to PostgreSQL results in a new backend process being spawned on the PostgreSQL server.
///
/// `PARADE_INDEX_MEMORY` is designed to act as a cache that persists for the lifetime of a
/// single backend process. When a client connects to PostgreSQL and triggers the extension's
/// functionality, this cache is initialized the first time it's accessed in that specific process.
///
/// In scenarios where connection pooling is used, such as by web servers maintaining
/// a pool of connections to PostgreSQL, the connections (and the associated backend processes)
/// are typically long-lived. While this cache initialization might happen once per connection,
/// it does not happen per query, leading to performance benefits for expensive operations.
///
/// It's also crucial to remember that this cache is NOT shared across different backend
/// processes. Each PostgreSQL backend process will have its own separate instance of
/// this cache, tied to its own lifecycle.
static mut PARADE_INDEX_MEMORY: Option<HashMap<String, ParadeIndex>> = None;

pub enum ParadeIndexKey {
    Number(i64),
}

impl TryFrom<&JsonBuilderValue> for ParadeIndexKey {
    type Error = Box<dyn Error>;

    fn try_from(value: &JsonBuilderValue) -> Result<Self, Self::Error> {
        match value {
            JsonBuilderValue::i16(v) => Ok(ParadeIndexKey::Number(*v as i64)),
            JsonBuilderValue::i32(v) => Ok(ParadeIndexKey::Number(*v as i64)),
            JsonBuilderValue::i64(v) => Ok(ParadeIndexKey::Number(*v)),
            JsonBuilderValue::u32(v) => Ok(ParadeIndexKey::Number(*v as i64)),
            JsonBuilderValue::u64(v) => Ok(ParadeIndexKey::Number(*v as i64)),
            _ => Err(format!("Unsupported conversion: {:#?}", value).into()),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct ParadeIndex {
    pub name: String,
    pub fields: HashMap<String, Field>,
    pub field_configs: ParadeOptionMap,
    pub key_field_name: String,
    #[serde(skip_serializing, deserialize_with = "ParadeIndex::deserialize_reader")]
    reader: IndexReader,
    #[serde(skip_serializing)]
    underlying_index: Index,
    #[serde(skip_serializing)]
    key_field: Field,
}

impl ParadeIndex {
    pub fn new(
        name: String,
        heap_relation: &PgRelation,
        options: PgBox<ParadeOptions>,
    ) -> Result<Self, Box<dyn Error>> {
        let dir = Self::get_data_directory(&name);
        let path = Path::new(&dir);

        if path.exists() {
            remove_dir_all(path).expect("failed to remove paradedb directory");
        }

        create_dir_all(path).expect("failed to create paradedb directory");

        let key_field_name = options.get_key_field();
        let result = Self::build_index_schema(heap_relation, &key_field_name, &options);
        let (schema, fields) = match result {
            Ok((s, f)) => (s, f),
            Err(e) => {
                panic!("{}", e);
            }
        };
        let settings = IndexSettings {
            docstore_compress_dedicated_thread: false, // Must run on single thread, or pgrx will panic
            ..Default::default()
        };

        let mut underlying_index = Index::builder()
            .schema(schema.clone())
            .settings(settings.clone())
            .create_in_dir(dir)
            .expect("failed to create index");

        let key_field = schema.get_field(&key_field_name).unwrap_or_else(|_| {
            panic!("error creating index: key_field '{key_field_name}' does not exist in schema",)
        });

        // Save the json_fields used to configure the index to disk.
        // We'll need to retrieve these along with the index.
        let mut field_configs: ParadeOptionMap = HashMap::new();

        for (field_name, options) in options.get_text_fields() {
            field_configs.insert(field_name, ParadeOption::Text(options));
        }

        for (field_name, options) in options.get_json_fields() {
            field_configs.insert(field_name, ParadeOption::Json(options));
        }

        for (field_name, options) in options.get_numeric_fields() {
            field_configs.insert(field_name, ParadeOption::Numeric(options));
        }

        for (field_name, options) in options.get_boolean_fields() {
            field_configs.insert(field_name, ParadeOption::Boolean(options));
        }

        Self::setup_tokenizers(&mut underlying_index, &field_configs);

        let reader = Self::reader(&underlying_index).unwrap_or_else(|_| {
            panic!("failed to create index reader while creating new index: {name}")
        });

        plog!(
            "creating ParadeIndex",
            json!({
                "name": name,
                "fields": fields,
                "field_configs": field_configs
            })
        );
        let new_self = Self {
            name: name.clone(),
            fields,
            field_configs,
            reader,
            underlying_index,
            key_field_name,
            key_field,
        };

        // Serialize ParadeIndex to disk so it can be initialized by other connections.
        new_self.to_disk();

        // Save a reference to this ParadeIndex so it can be re-used by this connection.
        unsafe {
            new_self.to_cached_index();
        }

        Ok(new_self)
    }

    fn setup_tokenizers(underlying_index: &mut Index, field_configs: &ParadeOptionMap) {
        underlying_index.set_tokenizers(create_tokenizer_manager(field_configs));
        underlying_index.set_fast_field_tokenizers(create_normalizer_manager());
    }

    fn reader(index: &Index) -> Result<IndexReader, TantivyError> {
        index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
    }

    unsafe fn from_cached_index(name: &str) -> Option<Self> {
        // This function needs to be unsafe because it accesses
        // data from a static mut variable.
        // If the cache has not been initialized for this process,
        // initialize it first.
        if PARADE_INDEX_MEMORY.is_none() {
            PARADE_INDEX_MEMORY = Some(HashMap::new());
            return None;
        }

        PARADE_INDEX_MEMORY.as_ref()?.get(name).cloned()
    }

    unsafe fn to_cached_index(&self) {
        if PARADE_INDEX_MEMORY.is_none() {
            PARADE_INDEX_MEMORY = Some(HashMap::new());
        }

        PARADE_INDEX_MEMORY
            .as_mut()
            .unwrap()
            .insert(self.name.clone(), self.clone());
    }

    pub fn from_index_name(name: &str) -> Self {
        unsafe {
            // First check cache to see if we can retrieve the index from memory.
            if let Some(new_self) = Self::from_cached_index(name) {
                return new_self;
            }
        }

        let new_self = Self::from_disk(name);

        // Since we've re-fetched the index, save it to the cache.
        unsafe {
            new_self.to_cached_index();
        }

        new_self
    }

    pub fn get_key_value(&self, document: &Document) -> ParadeIndexKey {
        let key_field_name = &self.key_field_name;
        let value = document.get_first(self.key_field).unwrap_or_else(|| {
            panic!("cannot find key field '{key_field_name}' on retrieved document")
        });

        match value {
            tantivy::schema::Value::U64(val) => ParadeIndexKey::Number(*val as i64),
            tantivy::schema::Value::I64(val) => ParadeIndexKey::Number(*val),
            _ => panic!("invalid type for parade index key in document"),
        }
    }

    pub fn insert_with_writer(
        &mut self,
        writer: &mut SingleSegmentIndexWriter,
        ctid: ItemPointerData,
        builder: JsonBuilder,
    ) {
        // This method is both an implemenation for `self.insert`, and used publicly
        // during index build, where we want to make sure that the same writer is used
        // for the entire build.
        let mut doc: Document = Document::new();
        for (col_name, value) in builder.values.iter() {
            let field_option = self.fields.get(col_name.trim_matches('"'));
            if let Some(field) = field_option {
                value.add_to_tantivy_doc(&mut doc, field);
            }
        }

        let field_option = self.fields.get("ctid");
        doc.add_u64(*field_option.unwrap(), item_pointer_to_u64(ctid));
        writer.add_document(doc).expect("failed to add document");
    }

    pub fn insert(&mut self, ctid: ItemPointerData, builder: JsonBuilder) {
        // We expect this method to be called during regular inserts (after index creation).
        // We need to create a new writer each time to avoid race conditions, and make sure
        // to reload the reader afterwards.
        let mut writer = self
            .single_segment_writer()
            .expect("Could not retrieve single segment writer for insert");
        self.insert_with_writer(&mut writer, ctid, builder);
        writer.commit().unwrap();
        self.reload();
    }

    pub fn bulk_delete(
        &self,
        mut stats_binding: PgBox<IndexBulkDeleteResult, AllocatedByPostgres>,
        callback: IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) -> PgBox<IndexBulkDeleteResult, AllocatedByPostgres> {
        let mut index_writer = self
            .underlying_index
            .writer(INDEX_TANTIVY_MEMORY_BUDGET)
            .unwrap(); // Adjust the size as
                       // necessary

        let schema = self.underlying_index.schema();
        let ctid_field = schema
            .get_field("ctid")
            .expect("Field 'ctid' not found in schema");

        let searcher = self.searcher();

        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc_id in 0..segment_reader.num_docs() {
                if let Ok(stored_fields) = store_reader.get(doc_id) {
                    if let Some(Value::U64(ctid_val)) = stored_fields.get_first(ctid_field) {
                        if let Some(actual_callback) = callback {
                            let mut ctid = pg_sys::ItemPointerData::default();
                            u64_to_item_pointer(*ctid_val, &mut ctid);

                            let should_delete =
                                unsafe { actual_callback(&mut ctid, callback_state) };

                            if should_delete {
                                let term_to_delete = Term::from_field_u64(ctid_field, *ctid_val);
                                index_writer.delete_term(term_to_delete);
                                stats_binding.pages_deleted += 1;
                            } else {
                                stats_binding.num_pages += 1;
                            }
                        }
                    }
                }
            }
        }

        index_writer
            .prepare_commit()
            .expect("could not prepare_commit");
        index_writer.commit().unwrap();
        stats_binding
    }

    pub fn query_parser(&self) -> QueryParser {
        QueryParser::for_index(
            &self.underlying_index,
            self.schema()
                .fields()
                .map(|(field, _)| field)
                .collect::<Vec<_>>(),
        )
    }

    pub fn scan_state(&self, config: &SearchConfig) -> TantivyScanState {
        self.reload();
        TantivyScanState::new(self, config)
    }

    pub fn schema(&self) -> Schema {
        self.underlying_index.schema()
    }

    pub fn searcher(&self) -> Searcher {
        self.reader.searcher()
    }

    pub fn single_segment_writer(&self) -> Result<SingleSegmentIndexWriter, TantivyError> {
        SingleSegmentIndexWriter::new(self.underlying_index.clone(), INDEX_TANTIVY_MEMORY_BUDGET)
    }

    pub fn writer(&self) -> Result<IndexWriter, TantivyError> {
        self.underlying_index.writer(INDEX_TANTIVY_MEMORY_BUDGET)
    }

    pub fn reload(&self) {
        self.reader.reload().unwrap();
    }

    pub fn garbage_collect_files(&self) {
        let index_writer = self
            .writer()
            .expect("Could not create writer to garbage collect files");

        index_writer
            .garbage_collect_files()
            .wait()
            .expect("Could not collect garbage");
    }

    fn get_data_directory(name: &str) -> String {
        unsafe {
            let option_name_cstr =
                CString::new("data_directory").expect("failed to create CString");
            let data_dir_str = String::from_utf8(
                CStr::from_ptr(pg_sys::GetConfigOptionByName(
                    option_name_cstr.as_ptr(),
                    std::ptr::null_mut(),
                    true,
                ))
                .to_bytes()
                .to_vec(),
            )
            .expect("Failed to convert C string to Rust string");

            format!("{}/{}/{}", data_dir_str, "paradedb", name)
        }
    }

    fn get_field_configs_path(name: &str) -> String {
        let dir_path = Self::get_data_directory(name);
        format!("{}_parade_field_configs.json", dir_path)
    }

    fn to_disk(&self) {
        let index_name = &self.name;
        let config_path = &Self::get_field_configs_path(index_name);
        let serialized_data = serde_json::to_string(self).unwrap_or_else(|err| {
            panic!("could not serialize index config for {index_name}: {err:?}")
        });
        let mut file = File::create(config_path).unwrap_or_else(|err| {
            panic!("could not create file to save index {index_name} at {config_path}: {err:?}")
        });

        file.write_all(serialized_data.as_bytes())
            .unwrap_or_else(|err| {
                panic!("could not write index for index {index_name} at {config_path}: {err:?}")
            });

        // Rust automatically flushes data to disk at the end of the scope,
        // so this call to "flush()" isn't strictly necessary.
        // We're doing it explicitly as a reminder in case we extend this method.
        file.flush().unwrap();
    }

    fn from_disk(index_name: &str) -> Self {
        let config_path = &Self::get_field_configs_path(index_name);
        let serialized_data = fs::read_to_string(config_path).unwrap_or_else(|err| {
            panic!("could not read index config for {index_name} from {config_path}: {err:?}")
        });

        serde_json::from_str(&serialized_data).unwrap_or_else(|err| {
            panic!("could not deserialize config from disk for index {index_name}: {err:?}");
        })
    }

    fn build_index_schema(
        heap_relation: &PgRelation,
        key_field_name: &str,
        options: &PgBox<ParadeOptions>,
    ) -> Result<(Schema, HashMap<String, Field>), String> {
        let tupdesc = heap_relation.tuple_desc();
        let mut schema_builder = Schema::builder();
        let mut fields: HashMap<String, Field> = HashMap::new();

        let text_fields = options.get_text_fields();
        let numeric_fields = options.get_numeric_fields();
        let boolean_fields = options.get_boolean_fields();
        let json_fields = options.get_json_fields();

        if text_fields.is_empty()
            && numeric_fields.is_empty()
            && boolean_fields.is_empty()
            && json_fields.is_empty()
        {
            return Err(
                "no text_fields, numeric_fields, boolean_fields, or json_fields were specified"
                    .to_string(),
            );
        }

        for (_, attribute) in tupdesc.iter().enumerate() {
            if attribute.is_dropped() {
                continue;
            }

            let attribute_type_oid = attribute.type_oid();
            let attname = attribute.name();
            let is_key_field = attname == key_field_name;
            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let base_oid = if array_type != pg_sys::InvalidOid {
                PgOid::from(array_type)
            } else {
                attribute_type_oid
            };

            let field = match &base_oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                        if is_key_field {
                            panic!("bm25 key field must be an integer type, received text")
                        } else {
                            text_fields.get(attname).map(|options| {
                                let text_options: TextOptions = (*options).into();
                                schema_builder.add_text_field(attname, text_options)
                            })
                        }
                    }
                    PgBuiltInOids::INT2OID
                    | PgBuiltInOids::INT4OID
                    | PgBuiltInOids::INT8OID
                    | PgBuiltInOids::OIDOID
                    | PgBuiltInOids::XIDOID => {
                        if is_key_field {
                            schema_builder
                                .add_i64_field(attname, INDEXED | STORED)
                                .into()
                        } else {
                            numeric_fields.get(attname).map(|options| {
                                let numeric_options: NumericOptions = (*options).into();
                                schema_builder.add_i64_field(attname, numeric_options)
                            })
                        }
                    }
                    PgBuiltInOids::FLOAT4OID
                    | PgBuiltInOids::FLOAT8OID
                    | PgBuiltInOids::NUMERICOID => {
                        if is_key_field {
                            panic!("bm25 key field must be an integer type, received float")
                        } else {
                            numeric_fields.get(attname).map(|options| {
                                let numeric_options: NumericOptions = (*options).into();
                                schema_builder.add_f64_field(attname, numeric_options)
                            })
                        }
                    }
                    PgBuiltInOids::BOOLOID => {
                        if is_key_field {
                            panic!("bm25 id column must be an integer type, received bool")
                        } else {
                            boolean_fields.get(attname).map(|options| {
                                let boolean_options: NumericOptions = (*options).into();
                                schema_builder.add_bool_field(attname, boolean_options)
                            })
                        }
                    }
                    PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => {
                        if is_key_field {
                            panic!("bm25 id column must be an integer type, received json")
                        } else {
                            json_fields.get(attname).map(|options| {
                                let json_options: JsonObjectOptions = (*options).into();
                                schema_builder.add_json_field(attname, json_options)
                            })
                        }
                    }
                    _ => None,
                },
                _ => None,
            };

            if let Some(valid_field) = field {
                fields.insert(attname.to_string(), valid_field);
            }
        }

        // "ctid" is a reserved column name in Postgres, so we don't need to worry about
        // creating a name conflict with a user-named column.
        let ctid_field = schema_builder.add_u64_field("ctid", INDEXED | STORED);
        fields.insert("ctid".to_string(), ctid_field);

        Ok((schema_builder.build(), fields))
    }
}

impl<'de> Deserialize<'de> for ParadeIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // A helper struct that lets use use the default serialization for most fields.
        #[derive(Deserialize)]
        struct ParadeIndexHelper {
            name: String,
            fields: HashMap<String, Field>,
            field_configs: ParadeOptionMap,
            key_field_name: String,
        }

        // Deserialize into the struct with automatic handling for most fields
        let ParadeIndexHelper {
            name,
            fields,
            field_configs,
            key_field_name,
        } = ParadeIndexHelper::deserialize(deserializer)?;

        let dir = Self::get_data_directory(&name);
        let mut underlying_index = Index::open_in_dir(dir).expect("failed to open index");
        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &field_configs);

        let schema = underlying_index.schema();
        let reader = Self::reader(&underlying_index).unwrap_or_else(|_| {
            panic!("failed to create index reader while retrieving index: {name}")
        });

        let key_field = schema.get_field(&key_field_name).unwrap_or_else(|_| {
            panic!(
                "error deserializing index: key_field '{key_field_name}' does not exist in schema",
            )
        });

        // Construct the ParadeIndex
        Ok(ParadeIndex {
            name,
            fields,
            field_configs,
            reader,
            underlying_index,
            key_field_name,
            key_field,
        })
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {

    use super::ParadeIndex;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_get_data_directory() {
        let dir_name = "thescore";
        let current_execution_dir = std::env::current_dir().unwrap();
        let expected = format!(
            "{}/paradedb/{dir_name}",
            current_execution_dir.to_str().unwrap()
        );
        let result = ParadeIndex::get_data_directory(dir_name);
        assert_eq!(result, expected);
    }

    #[pg_test]
    fn test_get_field_configs_path() {
        let name = "thescore";
        let current_execution_dir = std::env::current_dir().unwrap();
        let expected = format!(
            "{}/paradedb/{name}_parade_field_configs.json",
            current_execution_dir.to_str().unwrap()
        );
        let result = ParadeIndex::get_field_configs_path(name);
        assert_eq!(result, expected);
    }

    #[pg_test]
    #[should_panic]
    fn test_index_from_disk_panics() {
        let index_name = "tomwalker";
        ParadeIndex::from_disk(index_name);
    }

    #[pg_test]
    fn test_from_index_name() {
        Spi::run(SETUP_SQL).expect("failed to create index");
        let index_name = "idx_one_republic";
        let index = ParadeIndex::from_index_name(index_name);
        let fields = index.fields;
        assert_eq!(fields.len(), 8);
    }
}
