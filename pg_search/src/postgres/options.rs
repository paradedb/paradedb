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

use anyhow::{Context, Result};
use memoffset::*;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use std::collections::HashMap;
use std::ffi::CStr;

use crate::schema::{SearchFieldConfig, SearchFieldName};

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

static mut RELOPT_KIND_PDB: pg_sys::relopt_kind = 0;

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
    datetime_fields_offset: i32,
    key_field_offset: i32,
    uuid_offset: i32,
    memory_budget: i32,
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
extern "C" fn validate_key_field(value: *const std::os::raw::c_char) {
    cstr_to_rust_str(value);
}

#[pg_guard]
extern "C" fn validate_uuid(value: *const std::os::raw::c_char) {
    cstr_to_rust_str(value);
}

#[pg_guard]
extern "C" fn validate_memory_budget(value: *const std::os::raw::c_char) {
    let memory_budget_str = cstr_to_rust_str(value);
    if !memory_budget_str.is_empty() {
        memory_budget_str
            .parse::<i32>()
            .expect("Invalid memory budget value");
    }
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
pub unsafe extern "C" fn amoptions(
    reloptions: pg_sys::Datum,
    validate: bool,
) -> *mut pg_sys::bytea {
    let options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS] = [
        pg_sys::relopt_parse_elt {
            optname: "text_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, text_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "numeric_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, numeric_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "boolean_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, boolean_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "json_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, json_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "datetime_fields".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, datetime_fields_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "key_field".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, key_field_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "uuid".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, uuid_offset) as i32,
        },
        pg_sys::relopt_parse_elt {
            optname: "memory_budget".as_pg_cstr(),
            opttype: pg_sys::relopt_type_RELOPT_TYPE_STRING,
            offset: offset_of!(SearchIndexCreateOptions, memory_budget) as i32,
        },
    ];
    build_relopts(reloptions, validate, options)
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
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

// build_reloptions is not available when pg<13, so we need our own
#[cfg(feature = "pg12")]
unsafe fn build_relopts(
    reloptions: pg_sys::Datum,
    validate: bool,
    options: [pg_sys::relopt_parse_elt; NUM_REL_OPTS],
) -> *mut pg_sys::bytea {
    let mut n_options = 0;
    let p_options = pg_sys::parseRelOptions(reloptions, validate, RELOPT_KIND_PDB, &mut n_options);
    if n_options == 0 {
        return std::ptr::null_mut();
    }

    for relopt in std::slice::from_raw_parts_mut(p_options, n_options as usize) {
        relopt.gen.as_mut().unwrap().lockmode = pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE;
    }

    let rdopts = pg_sys::allocateReloptStruct(
        std::mem::size_of::<SearchIndexCreateOptions>(),
        p_options,
        n_options,
    );
    pg_sys::fillRelOptions(
        rdopts,
        std::mem::size_of::<SearchIndexCreateOptions>(),
        p_options,
        n_options,
        validate,
        options.as_ptr(),
        options.len() as i32,
    );
    pg_sys::pfree(p_options as void_mut_ptr);

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
        let config_map: HashMap<String, serde_json::Value> = json5::from_str(&serialized)
            .unwrap_or_else(|err| panic!("failed to deserialize field config: {err:?}"));

        config_map
            .into_iter()
            .map(|(field_name, field_config)| {
                (
                    field_name.clone().into(),
                    parser(field_config)
                        .context("failed to deserialize {field_name} config")
                        .unwrap(),
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

    pub fn get_datetime_fields(&self) -> Vec<(SearchFieldName, SearchFieldConfig)> {
        let config = self.get_str(self.datetime_fields_offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        Self::deserialize_config_fields(config, &SearchFieldConfig::date_from_json)
    }

    pub fn get_key_field(&self) -> Option<SearchFieldName> {
        let key_field = self.get_str(self.key_field_offset, "".to_string());
        if key_field.is_empty() {
            None
        } else {
            Some(key_field.into())
        }
    }

    pub fn get_uuid(&self) -> Option<String> {
        let uuid = self.get_str(self.uuid_offset, "".to_string());
        if uuid.is_empty() {
            None
        } else {
            Some(uuid)
        }
    }

    pub fn get_memory_budget(&self) -> i32 {
        self.memory_budget
    }

    fn get_str(&self, offset: i32, default: String) -> String {
        if offset == 0 {
            default
        } else {
            let opts = self as *const _ as void_ptr as usize;
            let value =
                unsafe { CStr::from_ptr((opts + offset as usize) as *const std::os::raw::c_char) };

            value.to_str().unwrap().to_owned()
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
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "numeric_fields".as_pg_cstr(),
        "JSON string specifying how numeric fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_numeric_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "boolean_fields".as_pg_cstr(),
        "JSON string specifying how boolean fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_boolean_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "json_fields".as_pg_cstr(),
        "JSON string specifying how JSON fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_json_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "datetime_fields".as_pg_cstr(),
        "JSON string specifying how date fields should be indexed".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_datetime_fields),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "key_field".as_pg_cstr(),
        "Column name as a string specify the unique identifier for a row".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_key_field),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "uuid".as_pg_cstr(),
        "Unique uuid for search index instance".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_uuid),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
    pg_sys::add_string_reloption(
        RELOPT_KIND_PDB,
        "memory_budget".as_pg_cstr(),
        "Memory budget in bytes for search index operations".as_pg_cstr(),
        std::ptr::null(),
        Some(validate_memory_budget),
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        {
            pg_sys::AccessExclusiveLock as pg_sys::LOCKMODE
        },
    );
}
