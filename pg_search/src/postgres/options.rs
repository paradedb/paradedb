// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::api::FieldName;
use crate::api::HashMap;
use crate::postgres::insert::DEFAULT_LAYER_SIZES;
use crate::postgres::utils::extract_field_attributes;
use crate::schema::IndexRecordOption;
use crate::schema::{SearchFieldConfig, SearchFieldType};

use anyhow::Result;
use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use serde_json::Map;
use std::ffi::CStr;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

/* ADDING OPTIONS
 * in init(), call pg_sys::add_{type}_reloption (check postgres docs for what args you need)
 * add the corresponding entries to SearchIndexOptionsData struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into SearchIndexOptionsData using PgBox
 * (because SearchIndexOptionsData is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind::Type = 0;

#[pg_guard]
extern "C-unwind" fn validate_text_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::text_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_numeric_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::numeric_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_boolean_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::boolean_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_json_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::json_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_range_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::range_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_datetime_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    deserialize_config_fields(json_str, &SearchFieldConfig::date_from_json);
}

#[pg_guard]
extern "C-unwind" fn validate_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }

    // Just ensure the config can be deserialized as json.
    let _: HashMap<String, serde_json::Value> = json5::from_str(&json_str)
        .unwrap_or_else(|err| panic!("failed to deserialize field config: {err:?}"));
}

#[pg_guard]
extern "C-unwind" fn validate_key_field(value: *const std::os::raw::c_char) {
    cstr_to_rust_str(value);
}

#[pg_guard]
extern "C-unwind" fn validate_layer_sizes(value: *const std::os::raw::c_char) {
    if value.is_null() {
        // a NULL value means we're to use whatever our defaults are
        return;
    }
    let cstr = unsafe { CStr::from_ptr(value) };
    let str = cstr.to_str().expect("`layer_sizes` must be valid UTF-8");

    let cnt = get_layer_sizes(str).count();

    // we require at least two layers
    assert!(cnt >= 2, "There must be at least 2 layers in `layer_sizes`");
}

fn get_layer_sizes(s: &str) -> impl Iterator<Item = u64> + use<'_> {
    s.split(",").map(|part| {
        unsafe {
            // just make sure postgres can parse this byte size
            u64::try_from(
                direct_function_call::<i64>(pg_sys::pg_size_bytes, &[part.into_datum()])
                    .expect("`pg_size_bytes()` should not return NULL"),
            )
            .ok()
            .filter(|b| b > &0)
            .expect("a single layer size must be greater than zero")
        }
    })
}

#[inline]
fn cstr_to_rust_str(value: *const std::os::raw::c_char) -> String {
    if value.is_null() {
        return "".to_string();
    }

    unsafe { CStr::from_ptr(value) }
        .to_str()
        .expect("failed to parse fields as utf-8")
        .to_string()
}

const NUM_REL_OPTS: usize = 8;
#[pg_guard]
pub unsafe extern "C-unwind" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "text_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, json_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "range_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, range_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "datetime_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, datetime_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "key_field".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, key_field_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "layer_sizes".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptionsData, layer_sizes_offset) as i32,
        },
    ];
    build_relopts(reloptions, validate, options)
}

unsafe fn build_relopts(
    reloptions: pg_sys::Datum,
    validate: bool,
    options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS],
) -> *mut pg_sys::bytea {
    let rdopts = pg_sys::build_reloptions(
        reloptions,
        validate,
        RELOPT_KIND_PDB,
        std::mem::size_of::<SearchIndexOptionsData>(), // TODO: proper size calculator
        options.as_ptr(),
        NUM_REL_OPTS as i32,
    );

    rdopts as *mut pg_sys::bytea
}

#[derive(Clone, Debug)]
pub struct SearchIndexOptions {
    layer_sizes: Vec<u64>,
    key_field_name: FieldName,
    text_configs: HashMap<FieldName, SearchFieldConfig>,
    numeric_configs: HashMap<FieldName, SearchFieldConfig>,
    boolean_configs: HashMap<FieldName, SearchFieldConfig>,
    json_configs: HashMap<FieldName, SearchFieldConfig>,
    range_configs: HashMap<FieldName, SearchFieldConfig>,
    datetime_configs: HashMap<FieldName, SearchFieldConfig>,
    relation_oid: pg_sys::Oid,
}

impl SearchIndexOptions {
    pub unsafe fn from_relation(indexrel: &PgRelation) -> Self {
        let data = SearchIndexOptionsData::from_relation(indexrel);
        let key_field_name = data.key_field_name();

        let text_configs = data.text_configs();
        for (field_name, config) in &text_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(t, SearchFieldType::Text(_) | SearchFieldType::Uuid(_))
            });
        }

        let numeric_configs = data.numeric_configs();
        for (field_name, config) in &numeric_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(
                    t,
                    SearchFieldType::I64(_) | SearchFieldType::U64(_) | SearchFieldType::F64(_)
                )
            });
        }

        let boolean_configs = data.boolean_configs();
        for (field_name, config) in &boolean_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(t, SearchFieldType::Bool(_))
            });
        }

        let json_configs = data.json_configs();
        for (field_name, config) in &json_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(t, SearchFieldType::Json(_))
            });
        }

        let range_configs = data.range_configs();
        for (field_name, config) in &range_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(t, SearchFieldType::Range(_))
            });
        }

        let datetime_configs = data.datetime_configs();
        for (field_name, config) in &datetime_configs {
            validate_field_config(field_name, &key_field_name, config, indexrel.oid(), |t| {
                matches!(t, SearchFieldType::Date(_))
            });
        }

        Self {
            layer_sizes: data.layer_sizes(),
            key_field_name,
            text_configs,
            numeric_configs,
            boolean_configs,
            json_configs,
            range_configs,
            datetime_configs,
            relation_oid: indexrel.oid(),
        }
    }

    pub fn layer_sizes(&self) -> Vec<u64> {
        self.layer_sizes.clone()
    }

    pub fn key_field_name(&self) -> FieldName {
        self.key_field_name.clone()
    }

    /// Returns either the config explicitly set in the CREATE INDEX WITH options,
    /// falling back to the default config for the field type.
    pub fn field_config_or_default(&self, field_name: &FieldName) -> SearchFieldConfig {
        let field_config = self.field_config(field_name);
        if let Some(config) = field_config {
            return config;
        }

        let field_type = match field_config.as_ref().and_then(|config| config.alias()) {
            Some(alias) => get_field_type(alias, self.relation_oid),
            None => get_field_type(field_name, self.relation_oid),
        };
        field_config.unwrap_or_else(|| field_type.default_config())
    }

    /// Returns the config only if it is explicitly set in the CREATE INDEX WITH options
    pub fn field_config(&self, field_name: &FieldName) -> Option<SearchFieldConfig> {
        if field_name.is_ctid() {
            return Some(ctid_field_config());
        }

        if field_name.root() == self.key_field_name.root() {
            return Some(key_field_config(&get_field_type(
                field_name,
                self.relation_oid,
            )));
        }

        self.text_configs
            .get(field_name)
            .cloned()
            .or_else(|| self.numeric_configs.get(field_name).cloned())
            .or_else(|| self.boolean_configs.get(field_name).cloned())
            .or_else(|| self.json_configs.get(field_name).cloned())
            .or_else(|| self.range_configs.get(field_name).cloned())
            .or_else(|| self.datetime_configs.get(field_name).cloned())
    }

    /// Returns a `HashMap` of aliased text field names and their configs.
    pub fn aliased_text_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.text_configs
            .clone()
            .into_iter()
            .filter(|(_field_name, config)| {
                if let Some(alias) = config.alias() {
                    assert!(matches!(
                        get_field_type(alias, self.relation_oid),
                        SearchFieldType::Text(_)
                    ));
                    true
                } else {
                    false
                }
            })
            .collect()
    }

    /// Returns a `HashMap` of aliased JSON field names and their configs.
    pub fn aliased_json_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.json_configs
            .clone()
            .into_iter()
            .filter(|(_field_name, config)| {
                if let Some(alias) = config.alias() {
                    assert!(matches!(
                        get_field_type(alias, self.relation_oid),
                        SearchFieldType::Json(_)
                    ));
                    true
                } else {
                    false
                }
            })
            .collect()
    }
}

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
struct SearchIndexOptionsData {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    text_fields_offset: i32,
    numeric_fields_offset: i32,
    boolean_fields_offset: i32,
    json_fields_offset: i32,
    range_fields_offset: i32,
    datetime_fields_offset: i32,
    key_field_offset: i32,
    layer_sizes_offset: i32,
}

impl SearchIndexOptionsData {
    pub unsafe fn from_relation(indexrel: &PgRelation) -> &Self {
        let mut ptr = indexrel.rd_options as *const Self;
        if ptr.is_null() {
            ptr = pg_sys::palloc0(std::mem::size_of::<Self>()) as *const Self;
        }
        ptr.as_ref().unwrap()
    }

    /// Returns the configured `layer_sizes`, split into a [`Vec<u64>`] of byte sizes.
    ///
    /// If none is applied to the index, the specified `default` sizes are used.
    pub fn layer_sizes(&self) -> Vec<u64> {
        let layer_sizes_str = self.get_str(self.layer_sizes_offset, Default::default());
        if layer_sizes_str.trim().is_empty() {
            return DEFAULT_LAYER_SIZES.to_vec();
        }
        get_layer_sizes(&layer_sizes_str).collect()
    }

    pub fn key_field_name(&self) -> FieldName {
        let key_field_name = self.get_str(self.key_field_offset, "".to_string());
        if key_field_name.is_empty() {
            panic!("key_field WITH option should be configured");
        }
        key_field_name.into()
    }

    pub fn text_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(self.text_fields_offset, &SearchFieldConfig::text_from_json)
    }

    pub fn numeric_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(
            self.numeric_fields_offset,
            &SearchFieldConfig::numeric_from_json,
        )
    }

    pub fn boolean_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(
            self.boolean_fields_offset,
            &SearchFieldConfig::boolean_from_json,
        )
    }

    pub fn json_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(self.json_fields_offset, &SearchFieldConfig::json_from_json)
    }

    pub fn range_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(
            self.range_fields_offset,
            &SearchFieldConfig::range_from_json,
        )
    }

    pub fn datetime_configs(&self) -> HashMap<FieldName, SearchFieldConfig> {
        self.deserialize_configs(
            self.datetime_fields_offset,
            &SearchFieldConfig::date_from_json,
        )
    }

    fn deserialize_configs(
        &self,
        offset: i32,
        parser: &dyn Fn(serde_json::Value) -> Result<SearchFieldConfig>,
    ) -> HashMap<FieldName, SearchFieldConfig> {
        let mut configs = HashMap::default();
        let config = self.get_str(offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized = deserialize_config_fields(config, parser);
            for (field_name, config) in deserialized.drain() {
                configs.insert(field_name, config);
            }
        }
        configs
    }

    fn get_str(&self, offset: i32, default: String) -> String {
        if offset == 0 {
            default
        } else {
            let opts = self as *const _ as void_ptr as usize;
            let value =
                unsafe { CStr::from_ptr((opts + offset as usize) as *const std::os::raw::c_char) };

            value
                .to_str()
                .expect("value should be valid utf-8")
                .to_owned()
        }
    }
}

// it adds the tokenizer option to the list of relation options so we can parse it in amoptions
pub unsafe fn init() {
    // adding our own relopt type because zombodb does, but one of the built-in Postgres ones might be more appropriate
    RELOPT_KIND_PDB = pg_sys::add_reloption_kind();
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "text_fields".as_pg_cstr(),
        "JSON string specifying how text fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_text_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "numeric_fields".as_pg_cstr(),
        "JSON string specifying how numeric fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_numeric_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "boolean_fields".as_pg_cstr(),
        "JSON string specifying how boolean fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_boolean_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "json_fields".as_pg_cstr(),
        "JSON string specifying how JSON fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_json_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "range_fields".as_pg_cstr(),
        "JSON string specifying how range fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_range_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "datetime_fields".as_pg_cstr(),
        "JSON string specifying how date fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_datetime_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "key_field".as_pg_cstr(),
        "Column name as a string specify the unique identifier for a row".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_key_field),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "layer_sizes".as_pg_cstr(),
        "The sizes of each segment merge layer".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_layer_sizes),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
}

/// As a SearchFieldConfig is an enum, for it to be correctly serialized the variant needs
/// to be present on the json object. This helper method will "wrap" the json object in
/// another object with the variant key, which is passed into the function. For example:
///
/// {"Text": { <actual_config> }}
///
/// This way, serde will know to deserialize the config as SearchFieldConfig::Text.
fn deserialize_config_fields(
    serialized: String,
    parser: &dyn Fn(serde_json::Value) -> Result<SearchFieldConfig>,
) -> HashMap<FieldName, SearchFieldConfig> {
    let config_map: Map<String, serde_json::Value> = serde_json::from_str(&serialized)
        .unwrap_or_else(|err| panic!("failed to deserialize field config: {err:?}"));

    config_map
        .into_iter()
        .map(|(field_name, field_config)| {
            (
                field_name.clone().into(),
                parser(field_config).unwrap_or_else(|err| {
                    panic!(
                        "field config should be valid for SearchFieldConfig::{field_name}: {err}"
                    )
                }),
            )
        })
        .collect()
}

fn key_field_config(field_type: &SearchFieldType) -> SearchFieldConfig {
    match field_type {
        SearchFieldType::I64(_) | SearchFieldType::U64(_) | SearchFieldType::F64(_) => {
            SearchFieldConfig::Numeric {
                indexed: true,
                fast: true,
            }
        }
        SearchFieldType::Text(_) | SearchFieldType::Uuid(_) => SearchFieldConfig::Text {
            indexed: true,
            fast: true,
            fieldnorms: false,

            // NB:  This should use the `SearchTokenizer::Keyword` tokenizer but for historical
            // reasons it uses the `SearchTokenizer::Raw` tokenizer but with the same filters
            // configuration as the `SearchTokenizer::Keyword` tokenizer.
            #[allow(deprecated)]
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::keyword().clone()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
            column: None,
        },
        SearchFieldType::Json(_) => SearchFieldConfig::Json {
            indexed: true,
            fast: true,
            fieldnorms: false,
            expand_dots: false,
            #[allow(deprecated)]
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
            column: None,
        },
        SearchFieldType::Range(_) => SearchFieldConfig::Range { fast: true },
        SearchFieldType::Bool(_) => SearchFieldConfig::Boolean {
            indexed: true,
            fast: true,
        },
        SearchFieldType::Date(_) => SearchFieldConfig::Date {
            indexed: true,
            fast: true,
        },
    }
}

fn ctid_field_config() -> SearchFieldConfig {
    SearchFieldConfig::Numeric {
        indexed: true,
        fast: true,
    }
}

fn get_attribute_oid(field_name: &str, relation_oid: pg_sys::Oid) -> Option<PgOid> {
    let index_relation = unsafe { PgRelation::open(relation_oid) };
    extract_field_attributes(&index_relation)
        .into_iter()
        .find(|(name, _)| name == field_name)
        .map(|(_, type_oid)| type_oid.into())
}

fn get_field_type(field_name: &str, relation_oid: pg_sys::Oid) -> SearchFieldType {
    let attribute_oid = get_attribute_oid(field_name, relation_oid)
        .unwrap_or_else(|| panic!("field type should have been set for `{}`", field_name));
    (&attribute_oid).try_into().unwrap()
}

fn validate_field_config(
    field_name: &FieldName,
    key_field_name: &FieldName,
    config: &SearchFieldConfig,
    relation_oid: pg_sys::Oid,
    matches: fn(&SearchFieldType) -> bool,
) {
    if field_name.is_ctid() {
        panic!("the name `ctid` is reserved by pg_search");
    }

    if field_name.root() == key_field_name.root() {
        panic!(
            "cannot override BM25 configuration for key_field '{}', you must use an aliased field name and 'column' configuration key",
            field_name
        );
    }

    if let Some(alias) = config.alias() {
        if get_attribute_oid(alias, relation_oid).is_none() {
            panic!(
                "the column `{}` referenced by the field configuration for '{}' does not exist",
                alias, field_name
            );
        }
    }

    let field_name = config.alias().unwrap_or(field_name);
    let field_type = get_field_type(field_name, relation_oid);
    if matches(&field_type) {
        return;
    }
    panic!("`{}` was configured with the wrong type", field_name);
}
