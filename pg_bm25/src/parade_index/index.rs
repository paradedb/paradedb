use pgrx::pg_sys::ItemPointerData;
use pgrx::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use shared::plog;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::Path;
use tantivy::{query::QueryParser, schema::*, Document, Index, IndexSettings, Searcher};
use tantivy::{IndexReader, IndexWriter, TantivyError};

use crate::index_access::options::ParadeOptions;
use crate::index_access::utils::{self, categorize_tupdesc, row_to_json, SearchConfig};
use crate::json::builder::{JsonBuilder, JsonBuilderValue};
use crate::parade_index::fields::{ParadeOption, ParadeOptionMap};
use crate::tokenizers::{create_normalizer_manager, create_tokenizer_manager};
use crate::WRITER;

use super::state::TantivyScanState;

const INDEX_TANTIVY_MEMORY_BUDGET: usize = 500_000_000;
const CACHE_NUM_BLOCKS: usize = 10;

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

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ParadeIndexKeyValue {
    Number(i64),
}

impl TryFrom<&JsonBuilderValue> for ParadeIndexKeyValue {
    type Error = Box<dyn Error>;

    fn try_from(value: &JsonBuilderValue) -> Result<Self, Self::Error> {
        match value {
            JsonBuilderValue::i16(v) => Ok(ParadeIndexKeyValue::Number(*v as i64)),
            JsonBuilderValue::i32(v) => Ok(ParadeIndexKeyValue::Number(*v as i64)),
            JsonBuilderValue::i64(v) => Ok(ParadeIndexKeyValue::Number(*v)),
            JsonBuilderValue::u32(v) => Ok(ParadeIndexKeyValue::Number(*v as i64)),
            JsonBuilderValue::u64(v) => Ok(ParadeIndexKeyValue::Number(*v as i64)),
            _ => Err(format!(
                "BM25 index key field must be an integer, received: {:#?}",
                value
            )
            .into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ParadeIndexKey {
    pub name: String,
    pub value: ParadeIndexKeyValue,
}

// impl ParadeIndexKey {
//     pub fn from_json_builder(name: &str, builder: &JsonBuilder) -> Result<Self, Box<dyn Error>> {
//         for (field_name, value) in &builder.values {
//             if field_name == &format!("\"{name}\"") {
//                 if let Ok(key_value) = ParadeIndexKeyValue::try_from(value) {
//                     return Ok(Self {
//                         name: field_name.into(),
//                         value: key_value,
//                     });
//                 }
//             }
//         }

//         Err(format!("could not parse key_field '{name}' from row").into())
//     }
// }

#[derive(Serialize, Clone)]
pub struct ParadeIndex {
    pub name: String,
    pub fields: HashMap<String, Field>,
    pub field_configs: ParadeOptionMap,
    pub key_field_name: String,
    pub data_directory: String,
    #[serde(skip_serializing)]
    pub reader: IndexReader,
    #[serde(skip_serializing)]
    pub key_field: Field,
    #[serde(skip_serializing)]
    pub ctid_field: Field,
    #[serde(skip_serializing)]
    underlying_index: Index,
}

impl ParadeIndex {
    pub fn new(
        name: String,
        heap_relation: &PgRelation,
        options: PgBox<ParadeOptions>,
    ) -> Result<&mut Self, Box<dyn Error>> {
        let data_directory = Self::data_directory(&name);
        let path = Path::new(&data_directory);

        if path.exists() {
            remove_dir_all(path).unwrap_or_else(|err| {
                panic!("failed to remove paradedb directory at {path:?}: {err:?}")
            });
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
            .create_in_dir(&data_directory)
            .expect("failed to create index");

        let key_field = schema.get_field(&key_field_name).unwrap_or_else(|_| {
            panic!("error creating index: key_field '{key_field_name}' does not exist in schema",)
        });

        let ctid_field = schema.get_field("ctid").unwrap_or_else(|_| {
            panic!("error deserializing index: ctid field does not exist in schema",)
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
            data_directory,
            key_field,
            ctid_field,
        };

        // Serialize ParadeIndex to disk so it can be initialized by other connections.
        new_self.to_disk();

        // Save a reference to this ParadeIndex so it can be re-used by this connection.
        unsafe {
            new_self.to_cached_index();
        }

        // We need to return the Self that is borrowed from the cache.
        let new_self_ref = Self::from_index_name(&name.to_string().as_ref());
        Ok(new_self_ref)
    }

    fn data_directory(name: &str) -> String {
        format!("{}/{}/{}", utils::get_data_directory(), "paradedb", name)
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

    unsafe fn from_cached_index(name: String) -> Option<&'static mut Self> {
        // This function needs to be unsafe because it accesses
        // data from a static mut variable.
        // If the cache has not been initialized for this process,
        // initialize it first.
        if PARADE_INDEX_MEMORY.is_none() {
            PARADE_INDEX_MEMORY = Some(HashMap::new());
            return None;
        }

        PARADE_INDEX_MEMORY.as_mut()?.get_mut(&name)
    }

    unsafe fn to_cached_index(self) {
        if PARADE_INDEX_MEMORY.is_none() {
            PARADE_INDEX_MEMORY = Some(HashMap::new());
        }

        PARADE_INDEX_MEMORY
            .as_mut()
            .unwrap()
            .insert(self.name.clone(), self);
    }

    pub fn from_index_name<'a>(name: &str) -> &'a mut Self {
        unsafe {
            let cached_self = Self::from_cached_index(name.into());
            if cached_self.is_some() {
                return cached_self.unwrap();
            }
        }

        let index_directory_path = Self::data_directory(&name);
        let new_self = Self::from_disk(&index_directory_path);

        // Since we've re-fetched the index, save it to the cache.
        unsafe {
            new_self.to_cached_index();
        }

        Self::from_index_name(name)
    }

    pub fn get_key_value(&self, document: &Document) -> ParadeIndexKeyValue {
        let key_field_name = &self.key_field_name;
        let value = document.get_first(self.key_field).unwrap_or_else(|| {
            panic!("cannot find key field '{key_field_name}' on retrieved document")
        });

        match value {
            tantivy::schema::Value::U64(val) => ParadeIndexKeyValue::Number(*val as i64),
            tantivy::schema::Value::I64(val) => ParadeIndexKeyValue::Number(*val),
            _ => panic!("invalid type for parade index key in document"),
        }
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
        self.reader.reload().unwrap();
        TantivyScanState::new(self, config)
    }

    pub fn schema(&self) -> Schema {
        self.underlying_index.schema()
    }

    pub fn searcher(&self) -> Searcher {
        self.reader.searcher()
    }

    /// Retrieve an owned writer for a given index. This is a static method, as
    /// we expect to be called from the writer process. The return type needs to
    /// be entirely owned by the new process, with no references.
    pub fn writer(index_directory_path: &str) -> Result<IndexWriter, TantivyError> {
        let parade_index = Self::from_disk(&index_directory_path);
        log!("GOT PARADE_INDEX at {index_directory_path}");
        parade_index
            .underlying_index
            .writer(INDEX_TANTIVY_MEMORY_BUDGET)
    }

    fn get_field_configs_path(index_directory_path: &str) -> String {
        format!("{}_parade_field_configs.json", index_directory_path)
    }

    fn to_disk(&self) {
        let index_name = &self.name;
        let config_path = &Self::get_field_configs_path(&self.data_directory);
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

    fn from_disk(index_directory_path: &str) -> Self {
        pgrx::log!("IN FROM DISK");
        let config_path = &Self::get_field_configs_path(index_directory_path);
        pgrx::log!("GOT CONFIG PATH");
        let serialized_data = fs::read_to_string(config_path).unwrap_or_else(|err| {
            panic!("could not read index config for {index_directory_path} from {config_path}: {err:?}")
        });
        pgrx::log!("GOT SERIALIZED DATA");
        serde_json::from_str(&serialized_data).unwrap_or_else(|err| {
            panic!(
                "could not deserialize config from disk for index {index_directory_path}: {err:?}"
            );
        })
    }

    pub fn commit(&self) {
        WRITER.share().commit(&self.name);
    }

    pub fn insert(&self, builder: JsonBuilder) {
        // Send the insert requests to the writer server.
        // Note that these will not be flused to disk until commit() is separately called.
        WRITER.share().insert(&self.name, builder);
        self.reader.reload().unwrap();
    }

    pub fn delete(&mut self, should_delete: impl Fn(*mut ItemPointerData) -> bool) -> (u32, u32) {
        let mut deleted: u32 = 0;
        let mut not_deleted: u32 = 0;
        let mut ctids_to_delete: Vec<u64> = vec![];

        for segment_reader in self.searcher().segment_readers() {
            let store_reader = segment_reader
                .get_store_reader(CACHE_NUM_BLOCKS)
                .expect("Failed to get store reader");

            for (delete, ctid) in (0..segment_reader.num_docs())
                .filter_map(|id| store_reader.get(id).ok())
                .filter_map(|doc| doc.get_first(self.ctid_field).cloned())
                .filter_map(|value| match value {
                    Value::U64(ctid_val) => Some(ctid_val),
                    _ => None,
                })
                .map(|ctid_val| {
                    let mut ctid = ItemPointerData::default();
                    u64_to_item_pointer(ctid_val, &mut ctid);
                    (should_delete(&mut ctid), ctid_val)
                })
            {
                if delete {
                    ctids_to_delete.push(ctid);
                    deleted += 1
                } else {
                    not_deleted += 1
                }
            }
        }

        // Send the delete requests to the writer server.
        // Note that these will not be flused to disk until commit() is separately called.
        WRITER
            .share()
            .delete(&self.name, self.ctid_field.clone(), ctids_to_delete);

        (deleted, not_deleted)
    }

    pub fn json_builder(
        &self,
        ctid: pg_sys::ItemPointerData,
        tupdesc: &PgTupleDesc,
        values: *mut pg_sys::Datum,
    ) -> JsonBuilder {
        let attributes = categorize_tupdesc(&tupdesc);
        let natts = tupdesc.natts as usize;
        let dropped = (0..tupdesc.natts as usize)
            .map(|i| tupdesc.get(i).unwrap().is_dropped())
            .collect::<Vec<bool>>();
        let values = unsafe { std::slice::from_raw_parts(values, 1) };
        let mut builder = JsonBuilder::new(self.key_field, self.fields.clone());
        // Insert the fields from the row
        unsafe {
            row_to_json(
                values[0],
                &tupdesc,
                natts,
                &dropped,
                &attributes,
                &mut builder,
            )
        };

        // Insert the ctid value
        builder.add_u64("ctid".into(), item_pointer_to_u64(ctid));

        builder
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

        for attribute in tupdesc.iter() {
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
            data_directory: String,
        }

        // Deserialize into the struct with automatic handling for most fields
        let ParadeIndexHelper {
            name,
            fields,
            field_configs,
            key_field_name,
            data_directory,
        } = ParadeIndexHelper::deserialize(deserializer)?;

        let mut underlying_index =
            Index::open_in_dir(&data_directory).expect("failed to open index");
        // We need to setup tokenizers again after retrieving an index from disk.
        Self::setup_tokenizers(&mut underlying_index, &field_configs);

        let schema = underlying_index.schema();
        let reader = Self::reader(&underlying_index).unwrap_or_else(|_| {
            panic!("failed to create index reader while retrieving index: {name}")
        });

        let key_field = schema.get_field(&key_field_name).unwrap_or_else(|_| {
            panic!(
                "error deserializing index: key field '{key_field_name}' does not exist in schema",
            )
        });

        let ctid_field = schema.get_field("ctid").unwrap_or_else(|_| {
            panic!("error deserializing index: ctid field does not exist in schema",)
        });

        // Construct the ParadeIndex
        Ok(ParadeIndex {
            name,
            fields,
            field_configs,
            reader,
            underlying_index,
            key_field_name,
            data_directory,
            key_field,
            ctid_field,
        })
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {

    use super::ParadeIndex;
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    // #[pg_test]
    // fn test_get_data_directory() {
    //     let dir_name = "thescore";
    //     let current_execution_dir = std::env::current_dir().unwrap();
    //     let expected = format!(
    //         "{}/paradedb/{dir_name}",
    //         current_execution_dir.to_str().unwrap()
    //     );
    //     let result = ParadeIndex::get_data_directory(dir_name);
    //     assert_eq!(result, expected);
    // }

    // #[pg_test]
    // fn test_get_field_configs_path() {
    //     let name = "thescore";
    //     let current_execution_dir = std::env::current_dir().unwrap();
    //     let expected = format!(
    //         "{}/paradedb/{name}_parade_field_configs.json",
    //         current_execution_dir.to_str().unwrap()
    //     );
    //     let result = ParadeIndex::get_field_configs_path(name);
    //     assert_eq!(result, expected);
    // }

    // #[pg_test]
    // #[should_panic]
    // fn test_index_from_disk_panics() {
    //     let index_name = "tomwalker";
    //     ParadeIndex::from_disk(index_name);
    // }

    #[pg_test]
    fn test_from_index_name() {
        Spi::run(SETUP_SQL).expect("failed to create index");
        let index_name = "one_republic_songs_bm25_index";
        let index = ParadeIndex::from_index_name(index_name);
        let fields = &index.fields;
        assert_eq!(fields.len(), 8);
    }
}
