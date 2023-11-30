use pgrx::pg_sys::{IndexBulkDeleteCallback, IndexBulkDeleteResult, ItemPointerData};
use pgrx::*;
use serde_json::json;
use shared::plog;
use std::collections::HashMap;
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fs::{self, create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::Path;
use tantivy::{
    query::{Query, QueryParser},
    schema::*,
    DocAddress, Document, Index, IndexSettings, Score, Searcher, Term,
};
use tantivy::{IndexReader, IndexWriter, SingleSegmentIndexWriter, TantivyError};

use crate::index_access::options::ParadeOptions;
use crate::json::builder::JsonBuilder;
use crate::parade_index::fields::{
    ParadeFieldConfigDeserializedResult, ParadeFieldConfigSerialized,
    ParadeFieldConfigSerializedResult, ParadeOption, ParadeOptionMap,
};
use crate::tokenizers::{create_normalizer_manager, create_tokenizer_manager};

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

pub struct TantivyScanState {
    pub schema: Schema,
    pub query: Box<dyn Query>,
    pub query_parser: QueryParser,
    pub searcher: Searcher,
    pub iterator: *mut std::vec::IntoIter<(Score, DocAddress)>,
}

#[derive(Clone)]
pub struct ParadeIndex {
    pub name: String,
    pub fields: HashMap<String, Field>,
    pub field_configs: ParadeOptionMap,
    reader: IndexReader,
    underlying_index: Index,
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

        let result = Self::build_index_schema(heap_relation, &options);
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

        Self::write_index_field_configs(&name, &field_configs)?;
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
        };
        unsafe {
            new_self.to_cached_index(name);
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

    unsafe fn to_cached_index(&self, name: String) {
        if PARADE_INDEX_MEMORY.is_none() {
            PARADE_INDEX_MEMORY = Some(HashMap::new());
        }

        PARADE_INDEX_MEMORY
            .as_mut()
            .unwrap()
            .insert(name, self.clone());
    }

    pub fn from_index_name(name: String) -> Self {
        unsafe {
            // First check cache to see if we can retrieve the index from memory.
            if let Some(new_self) = Self::from_cached_index(&name) {
                return new_self;
            }
        }

        let dir = Self::get_data_directory(&name);

        let mut underlying_index = Index::open_in_dir(dir).expect("failed to open index");
        let schema = underlying_index.schema();

        let fields = schema
            .fields()
            .map(|field| {
                let (field, entry) = field;
                (entry.name().to_string(), field)
            })
            .collect();

        let field_configs =
            Self::read_index_field_configs(&name).expect("failed to open index field configs");

        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &field_configs);

        let reader = Self::reader(&underlying_index).unwrap_or_else(|_| {
            panic!("failed to create index reader while retrieving index: {name}")
        });

        let new_self = Self {
            name: name.clone(),
            fields,
            field_configs,
            reader,
            underlying_index,
        };

        // Since we've re-fetched the index, save it to the cache.
        unsafe {
            new_self.to_cached_index(name);
        }

        new_self
    }

    pub fn insert_with_writer(
        &mut self,
        writer: &mut SingleSegmentIndexWriter,
        heap_tid: ItemPointerData,
        builder: JsonBuilder,
    ) {
        // This method is both an implemenation for `self.insert`, and used publicly
        // during index build, where we want to make sure that the same writer is used
        // for the entire build.
        let mut doc: Document = Document::new();
        for (col_name, value) in builder.values {
            let field_option = self.fields.get(col_name.trim_matches('"'));
            if let Some(field) = field_option {
                value.add_to_tantivy_doc(&mut doc, field);
            }
        }

        let field_option = self.fields.get("heap_tid");
        doc.add_u64(*field_option.unwrap(), item_pointer_to_u64(heap_tid));
        writer.add_document(doc).expect("failed to add document");
    }

    pub fn insert(&mut self, heap_tid: ItemPointerData, builder: JsonBuilder) {
        // We expect this method to be called during regular inserts (after index creation).
        // We need to create a new writer each time to avoid race conditions, and make sure
        // to reload the reader afterwards.
        let mut writer = self
            .single_segment_writer()
            .expect("Could not retrieve single segment writer for insert");
        self.insert_with_writer(&mut writer, heap_tid, builder);
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
        let heap_tid_field = schema
            .get_field("heap_tid")
            .expect("Field 'heap_tid' not found in schema");

        let searcher = self.searcher();

        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for doc_id in 0..segment_reader.num_docs() {
                if let Ok(stored_fields) = store_reader.get(doc_id) {
                    if let Some(Value::U64(heap_tid_val)) = stored_fields.get_first(heap_tid_field)
                    {
                        if let Some(actual_callback) = callback {
                            let mut heap_tid = pg_sys::ItemPointerData::default();
                            u64_to_item_pointer(*heap_tid_val, &mut heap_tid);

                            let should_delete =
                                unsafe { actual_callback(&mut heap_tid, callback_state) };

                            if should_delete {
                                let term_to_delete =
                                    Term::from_field_u64(heap_tid_field, *heap_tid_val);
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

    pub fn scan(&self) -> TantivyScanState {
        self.reload();
        let schema = self.underlying_index.schema();

        let searcher = self.searcher();

        let query_parser = QueryParser::for_index(
            &self.underlying_index,
            schema.fields().map(|(field, _)| field).collect::<Vec<_>>(),
        );
        let empty_query = query_parser.parse_query("").unwrap();

        TantivyScanState {
            schema,
            query: empty_query,
            query_parser,
            searcher,
            iterator: std::ptr::null_mut(),
        }
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

    fn serialize_index_field_configs(
        field_configs: &ParadeOptionMap,
    ) -> ParadeFieldConfigSerializedResult {
        serde_json::to_string(&field_configs)
    }

    fn deserialize_index_field_configs(
        serialized_data: ParadeFieldConfigSerialized,
    ) -> ParadeFieldConfigDeserializedResult {
        serde_json::from_str(&serialized_data)
    }

    fn write_index_field_configs(
        index_name: &str,
        field_configs: &ParadeOptionMap,
    ) -> Result<(), Box<dyn Error>> {
        // Serialize the entire HashMap into a format writable to disk.
        let serialized_data = Self::serialize_index_field_configs(field_configs)?;
        let config_path = Self::get_field_configs_path(index_name);
        let mut file = File::create(config_path)?;

        file.write_all(serialized_data.as_bytes())?;
        // Rust automatically flushes data to disk at the end of the scope,
        // so this call to "flush()" isn't strictly necessary.
        // We're doing it explicitly as a reminder in case we extend this method.
        file.flush().unwrap();

        Ok(())
    }

    fn read_index_field_configs(index_name: &str) -> Result<ParadeOptionMap, Box<dyn Error>> {
        let config_path = Self::get_field_configs_path(index_name);

        let serialized_data = fs::read_to_string(config_path)?;

        // Deserialize the data from disk back into a HashMap<String, ParadeFieldConfig>.
        let deserialized_data = Self::deserialize_index_field_configs(serialized_data)?;

        Ok(deserialized_data)
    }

    fn build_index_schema(
        heap_relation: &PgRelation,
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
            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let base_oid = if array_type != pg_sys::InvalidOid {
                PgOid::from(array_type)
            } else {
                attribute_type_oid
            };

            let field = match &base_oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                        text_fields.get(attname).map(|options| {
                            let text_options: TextOptions = (*options).into();
                            schema_builder.add_text_field(attname, text_options)
                        })
                    }
                    PgBuiltInOids::INT2OID
                    | PgBuiltInOids::INT4OID
                    | PgBuiltInOids::INT8OID
                    | PgBuiltInOids::OIDOID
                    | PgBuiltInOids::XIDOID => numeric_fields.get(attname).map(|options| {
                        let numeric_options: NumericOptions = (*options).into();
                        schema_builder.add_i64_field(attname, numeric_options)
                    }),
                    PgBuiltInOids::FLOAT4OID
                    | PgBuiltInOids::FLOAT8OID
                    | PgBuiltInOids::NUMERICOID => numeric_fields.get(attname).map(|options| {
                        let numeric_options: NumericOptions = (*options).into();
                        schema_builder.add_f64_field(attname, numeric_options)
                    }),
                    PgBuiltInOids::BOOLOID => boolean_fields.get(attname).map(|options| {
                        let boolean_options: NumericOptions = (*options).into();
                        schema_builder.add_bool_field(attname, boolean_options)
                    }),
                    PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => {
                        json_fields.get(attname).map(|options| {
                            let json_options: JsonObjectOptions = (*options).into();
                            schema_builder.add_json_field(attname, json_options)
                        })
                    }
                    _ => None,
                },
                _ => None,
            };

            if let Some(valid_field) = field {
                fields.insert(attname.to_string(), valid_field);
            }
        }

        let field = schema_builder.add_u64_field("heap_tid", INDEXED | STORED);
        fields.insert("heap_tid".to_string(), field);

        Ok((schema_builder.build(), fields))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::{collections::HashMap, fs::OpenOptions, io::BufReader};

    use crate::parade_index::fields::{ParadeOption, ParadeTextOptions};

    use std::error::Error;

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
    fn test_serialize_index_field_configs() -> serde_json::Result<()> {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "fieldnorms": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let text_option: ParadeTextOptions =
            serde_json::from_str(json).expect("failed to serialize text options");
        let options = ParadeOption::Text(text_option);
        let mut field_configs = HashMap::new();
        field_configs.insert("onerepublic".to_string(), options);

        let result = ParadeIndex::serialize_index_field_configs(&field_configs)?;
        let expected = r#"{"onerepublic":{"Text":{"indexed":true,"fast":false,"stored":true,"fieldnorms":true,"tokenizer":{"type":"default"},"record":"basic","normalizer":"raw"}}}"#;

        assert_eq!(result, expected);
        Ok(())
    }

    #[pg_test]
    fn test_deserialize_index_field_configs() -> serde_json::Result<()> {
        let json = r#"{
           "hozier": {
                "Text": {
                    "indexed": true,
                    "fast": false,
                    "stored": true,
                    "fieldnorms": true,
                    "type": "default",
                    "record": "basic",
                    "normalizer": "raw"
                }
           }
        }"#;
        let result = ParadeIndex::deserialize_index_field_configs(json.to_string())?;
        let text_option: ParadeTextOptions = serde_json::from_str(json).unwrap();
        let options = ParadeOption::Text(text_option);
        let mut expected = HashMap::new();
        expected.insert("hozier".to_string(), options);

        let res_option = result.get("hoszier");
        let exp_option = expected.get("hozier");
        assert_ne!(res_option.is_some(), exp_option.is_some());

        Ok(())
    }

    fn make_field_config() -> HashMap<String, ParadeOption> {
        let json = r#"{
            "indexed": true,
            "fast": false,
            "stored": true,
            "fieldnorms": true,
            "type": "default",
            "record": "basic",
            "normalizer": "raw"
        }"#;
        let text_option: ParadeTextOptions = serde_json::from_str(json).unwrap();
        let options = ParadeOption::Text(text_option);
        let mut field_configs = HashMap::new();
        field_configs.insert("onerepublic".to_string(), options);
        field_configs
    }

    #[pg_test]
    fn test_write_index_field_configs() -> Result<(), Box<dyn Error>> {
        let index_name = "lorde";
        let field_configs = make_field_config();
        ParadeIndex::write_index_field_configs(index_name, &field_configs)?;

        let current_execution_dir = std::env::current_dir().unwrap();
        let index_field_location = format!(
            "{}/paradedb/{index_name}_parade_field_configs.json",
            current_execution_dir.to_str().unwrap()
        );

        let index_field_path = std::path::Path::new(&index_field_location);
        assert!(index_field_path.exists());

        let file = OpenOptions::new().read(true).open(index_field_path)?;
        let reader = BufReader::new(file);
        let result: HashMap<String, ParadeOption> = serde_json::from_reader(reader)?;

        assert_eq!(
            field_configs.get("onerepublic").is_some(),
            result.get("onerepublic").is_some()
        );

        Ok(())
    }

    #[pg_test]
    fn test_read_index_field_configs() {
        let index_name = "tomwalker";
        let result = ParadeIndex::read_index_field_configs(index_name);
        assert!(result.is_err());
    }

    #[pg_test]
    fn test_from_index_name() {
        Spi::run(SETUP_SQL).expect("failed to create index");
        let index_name = "idx_one_republic";
        let index = ParadeIndex::from_index_name(index_name.to_string());
        let fields = index.fields;
        assert_eq!(fields.len(), 7);
    }
}
