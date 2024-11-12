// Copyright (c) 2023-2024 Retake, Inc.
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

use anyhow::Result;
use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use std::collections::HashMap;
use std::ffi::CStr;

use crate::schema::{SearchFieldConfig, SearchFieldName, SearchFieldType};

/* ADDING OPTIONS
 * in init(), call pg_sys::add_{type}_reloption (check postgres docs for what args you need)
 * add the corresponding entries to SearchIndexCreateOptions struct definition
 * in amoptions(), add a relopt_parse_elt entry to the options array and change NUM_REL_OPTS
 * Note that for string options, postgres will give you the offset of the string, and you have to read the string
 * yourself (see get_tokenizer)
*/

/* READING OPTIONS
 * options are placed in relation.rd_options
 * As in ambuild(), cast relation.rd_options into SearchIndexCreateOptions using PgBox
 * (because SearchIndexCreateOptions is a postgres-allocated object) and use getters and setters
*/

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind::Type = 0;

// Postgres handles string options by placing each option offset bytes from the start of rdopts and
// plops the offset in the struct
#[repr(C)]
pub struct SearchIndexCreateOptions {
    // varlena header (needed bc postgres treats this as bytea)
    vl_len_: i32,
    text_fields_offset: i32,
    numeric_fields_offset: i32,
    boolean_fields_offset: i32,
    json_fields_offset: i32,
    range_fields_offset: i32,
    datetime_fields_offset: i32,
    fields_offset: i32,
    key_field_offset: i32,
    target_segment_count: i32,
    merge_on_insert: bool,
}

#[pg_guard]
extern "C" fn validate_text_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::text_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_numeric_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::numeric_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_boolean_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::boolean_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_json_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::json_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_range_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::range_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_datetime_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }
    SearchIndexCreateOptions::deserialize_config_fields(
        json_str,
        &SearchFieldConfig::date_from_json,
    );
}

#[pg_guard]
extern "C" fn validate_fields(value: *const std::os::raw::c_char) {
    let json_str = cstr_to_rust_str(value);
    if json_str.is_empty() {
        return;
    }

    // Just ensure the config can be deserialized as json.
    let _: HashMap<String, serde_json::Value> = json5::from_str(&json_str)
        .unwrap_or_else(|err| panic!("failed to deserialize field config: {err:?}"));
}

#[pg_guard]
extern "C" fn validate_key_field(value: *const std::os::raw::c_char) {
    cstr_to_rust_str(value);
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

const NUM_REL_OPTS: usize = 10;
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "text_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, json_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "range_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, range_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "datetime_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, datetime_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "key_field".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, key_field_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "target_segment_count".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_INT,
            offset: offset_of!(SearchIndexCreateOptions, target_segment_count) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "merge_on_insert".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_BOOL,
            offset: offset_of!(SearchIndexCreateOptions, merge_on_insert) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type::RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, fields_offset) as i32,
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
        std::mem::size_of::<SearchIndexCreateOptions>(), // TODO: proper size calculator
        options.as_ptr(),
        NUM_REL_OPTS as i32,
    );

    rdopts as *mut pg_sys::bytea
}

impl SearchIndexCreateOptions {
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
    ) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config_map: HashMap<String, serde_json::Value> = serde_json::from_str(&serialized)
            .unwrap_or_else(|_| {
                panic!("failed to deserialize field config: invalid JSON string: {serialized}")
            });
        config_map
            .into_iter()
            .map(|(field_name, field_config)| {
                (
                    field_name.clone().into(),
                    parser(field_config)
                        .expect("field config should be valid for SearchFieldConfig::{field_name}"),
                )
            })
            .collect()
    }

    pub fn get_text_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.text_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::text_from_json)
    }

    pub fn get_numeric_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.numeric_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::numeric_from_json)
    }

    pub fn get_boolean_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.boolean_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::boolean_from_json)
    }

    pub fn get_json_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.json_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::json_from_json)
    }

    pub fn get_range_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.range_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::range_from_json)
    }

    pub fn get_datetime_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.datetime_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::date_from_json)
    }

    pub fn get_fields(
        &self,
        heaprel: &PgRelation,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)> {
        // Create a map from column name to column type. We'll use this to verify that index
        // configurations passed by the user reference the correct types for each column.
        let name_type_map: HashMap<String, SearchFieldType> = heaprel
            .tuple_desc()
            .into_iter()
            .filter_map(|attribute| {
                let attname = attribute.name();
                let attribute_type_oid = attribute.type_oid();
                let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
                let base_oid = if array_type != pg_sys::InvalidOid {
                    PgOid::from(array_type)
                } else {
                    attribute_type_oid
                };
                if let Ok(search_field_type) = SearchFieldType::try_from(&base_oid) {
                    Some((attname.into(), search_field_type))
                } else {
                    None
                }
            })
            .collect();

        let config = self.get_str(self.fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }

        let config_map: HashMap<String, serde_json::Value> = serde_json::from_str(&config)
            .unwrap_or_else(|_| {
                panic!("failed to deserialize field config: invalid JSON string: {config}")
            });

        config_map
            .into_iter()
            .map(|(field_name, field_config)| {
                let field_type = name_type_map
                    .get(&field_name)
                    .expect("must be able to lookup field type by name");

                (
                    field_name.clone().into(),
                    match field_type {
                        SearchFieldType::Text => SearchFieldConfig::text_from_json(field_config),
                        SearchFieldType::I64 => SearchFieldConfig::numeric_from_json(field_config),
                        SearchFieldType::F64 => SearchFieldConfig::numeric_from_json(field_config),
                        SearchFieldType::U64 => SearchFieldConfig::numeric_from_json(field_config),
                        SearchFieldType::Bool => SearchFieldConfig::boolean_from_json(field_config),
                        SearchFieldType::Json => SearchFieldConfig::json_from_json(field_config),
                        SearchFieldType::Date => SearchFieldConfig::date_from_json(field_config),
                        SearchFieldType::Range => SearchFieldConfig::range_from_json(field_config),
                    }
                    .expect("field config should be valid for SearchFieldConfig::{field_name}"),
                    *field_type,
                )
            })
            .collect()
    }

    pub fn get_key_field(&self) -> Option<SearchFieldName> {
        let key_field = self.get_str(self.key_field_offset, "".to_string());
        if key_field.is_empty() {
            None
        } else {
            Some(key_field.into())
        }
    }

    pub fn target_segment_count(&self) -> usize {
        self.target_segment_count as usize
    }

    pub fn merge_on_insert(&self) -> bool {
        self.merge_on_insert
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
    pg_sys::add_int_reloption(
        RELOPT_KIND_PDB,
        "target_segment_count".as_pg_cstr(),
        "The minimum number of segments the index should try to maintain".as_pg_cstr(),
        std::thread::available_parallelism()
            .expect("failed to get available_parallelism")
            .get()
            .try_into()
            .expect("your computer should have a reasonable CPU count"),
        1,
        i32::MAX,
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_bool_reloption(
        RELOPT_KIND_PDB,
        "merge_on_insert".as_pg_cstr(),
        "Merge segments immediately after rows are inserted into the index".as_pg_cstr(),
        true,
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "fields".as_pg_cstr(),
        "JSON string specifying how date fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_fields),
        pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE,
    );
}
