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

use crate::api::index::FieldName;
use crate::api::HashMap;
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
 * add the corresponding entries to SearchIndexOptions struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into SearchIndexOptions using PgBox
 * (because SearchIndexOptions is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind::Type = 0;

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
pub struct SearchIndexOptions {
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
            offset: offset_of!(SearchIndexOptions, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, json_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "range_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, range_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "datetime_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, datetime_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "key_field".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, key_field_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "layer_sizes".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexOptions, layer_sizes_offset) as i32,
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
        std::mem::size_of::<SearchIndexOptions>(), // TODO: proper size calculator
        options.as_ptr(),
        NUM_REL_OPTS as i32,
    );

    rdopts as *mut pg_sys::bytea
}

impl SearchIndexOptions {
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
    pub fn layer_sizes(&self, default: &[u64]) -> Vec<u64> {
        let layer_sizes_str = self.get_str(self.layer_sizes_offset, Default::default());
        if layer_sizes_str.trim().is_empty() {
            return default.to_vec();
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

    pub fn field_config_or_default(
        &self,
        field_name: &FieldName,
        relation_oid: pg_sys::Oid,
    ) -> SearchFieldConfig {
        let field_config = self.field_config(field_name, relation_oid);
        if let Some(config) = field_config {
            return config;
        }

        let field_type = match field_config.as_ref().and_then(|config| config.alias()) {
            Some(alias) => self.aliased_field_type(alias, relation_oid),
            None => self.field_type(field_name, relation_oid),
        };
        field_config.unwrap_or_else(|| field_type.default_config())
    }

    pub fn field_config(
        &self,
        field_name: &FieldName,
        relation_oid: pg_sys::Oid,
    ) -> Option<SearchFieldConfig> {
        if field_name.is_ctid() {
            return Some(ctid_field_config());
        }

        if field_name.root() == self.key_field_name().root() {
            return Some(key_field_config(&self.field_type(field_name, relation_oid)));
        }

        // Text fields
        let config = self.get_str(self.text_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::text_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        // Numeric fields
        let config = self.get_str(self.numeric_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::numeric_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        // Boolean fields
        let config = self.get_str(self.boolean_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::boolean_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        // JSON fields
        let config = self.get_str(self.json_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::json_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        // Range fields
        let config = self.get_str(self.range_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::json_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        // Date/time fields
        let config = self.get_str(self.datetime_fields_offset, "".to_string());
        if !config.is_empty() {
            let mut deserialized =
                deserialize_config_fields(config, &SearchFieldConfig::date_from_json);
            if let Some(config) = deserialized.remove(field_name) {
                return Some(config);
            }
        }

        None
    }

    pub fn get_aliased_text_configs(
        &self,
        relation_oid: pg_sys::Oid,
    ) -> HashMap<FieldName, SearchFieldConfig> {
        let config = self.get_str(self.text_fields_offset, "".to_string());
        if config.is_empty() {
            return HashMap::default();
        }

        deserialize_config_fields(config, &SearchFieldConfig::text_from_json)
            .into_iter()
            .filter(|(_field_name, config)| {
                if let Some(alias) = config.alias() {
                    assert!(matches!(
                        self.aliased_field_type(alias, relation_oid),
                        SearchFieldType::Text(_)
                    ));
                    true
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn get_aliased_json_configs(
        &self,
        relation_oid: pg_sys::Oid,
    ) -> HashMap<FieldName, SearchFieldConfig> {
        let config = self.get_str(self.json_fields_offset, "".to_string());
        if config.is_empty() {
            return HashMap::default();
        }
        deserialize_config_fields(config, &SearchFieldConfig::json_from_json)
            .into_iter()
            .filter(|(_field_name, config)| {
                if let Some(alias) = config.alias() {
                    assert!(matches!(
                        self.aliased_field_type(alias, relation_oid),
                        SearchFieldType::Json(_)
                    ));
                    true
                } else {
                    false
                }
            })
            .collect()
    }

    fn aliased_field_type(&self, alias: &str, relation_oid: pg_sys::Oid) -> SearchFieldType {
        if alias == self.key_field_name().root() {
            panic!("key field cannot be aliased");
        }

        let index_relation = unsafe { PgRelation::open(relation_oid) };
        let tuple_desc = index_relation.tuple_desc();
        let attribute_oid = tuple_desc
            .iter()
            .find(|attribute| attribute.name() == alias)
            .unwrap()
            .type_oid();
        (&attribute_oid).try_into().unwrap()
    }

    fn field_type(&self, field_name: &FieldName, relation_oid: pg_sys::Oid) -> SearchFieldType {
        let index_relation = unsafe { PgRelation::open(relation_oid) };
        let tuple_desc = index_relation.tuple_desc();
        let attribute_oid = tuple_desc
            .iter()
            .find(|attribute| attribute.name() == field_name.root())
            .unwrap()
            .type_oid();
        (&attribute_oid).try_into().unwrap()
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
                parser(field_config).unwrap_or_else(|_| {
                    panic!("field config should be valid for SearchFieldConfig::{field_name}")
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
        SearchFieldType::Text(_) => SearchFieldConfig::Text {
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
        SearchFieldType::Uuid(_) => SearchFieldConfig::default_uuid(),
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
