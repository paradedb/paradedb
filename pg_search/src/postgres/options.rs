// Copyright (c) 2023-2025 Retake, Inc.
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
use pgrx::pg_sys::{AsPgCStr, PgNode};
use pgrx::*;
use rustc_hash::FxHashMap;
use serde_json::Map;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::CStr;
use tokenizers::{manager::SearchTokenizerFilters, SearchNormalizer, SearchTokenizer};

use crate::schema::{IndexRecordOption, SearchFieldConfig, SearchFieldName, SearchFieldType};

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
    key_field_offset: i32,
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

const NUM_REL_OPTS: usize = 7;
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
    pub unsafe fn from_relation(indexrel: &PgRelation) -> &Self {
        let mut ptr = indexrel.rd_options as *const Self;
        if ptr.is_null() {
            ptr = pg_sys::palloc0(std::mem::size_of::<Self>()) as *const Self;
        }
        ptr.as_ref().unwrap()
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
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        let config_map: Map<String, serde_json::Value> = serde_json::from_str(&serialized)
            .unwrap_or_else(|err| panic!("failed to deserialize field config: {err:?}"));

        config_map
            .into_iter()
            .map(|(field_name, field_config)| {
                (
                    field_name.clone().into(),
                    parser(field_config)
                        .expect("field config should be valid for SearchFieldConfig::{field_name}"),
                    None,
                )
            })
            .collect()
    }

    fn get_fields_at_offset(
        &self,
        offset: i32,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
        parser: &dyn Fn(serde_json::Value) -> Result<SearchFieldConfig>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        let config = self.get_str(offset, "".to_string());
        if config.is_empty() {
            return Vec::new();
        }
        let mut configs = Self::deserialize_config_fields(config, parser);
        self.validate_fields_and_set_types(key_field_name, attributes, &mut configs);
        configs
    }

    fn get_text_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.text_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::text_from_json,
        )
    }

    fn get_numeric_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.numeric_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::numeric_from_json,
        )
    }

    fn get_boolean_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.boolean_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::boolean_from_json,
        )
    }

    fn get_json_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.json_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::json_from_json,
        )
    }

    fn get_range_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.range_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::range_from_json,
        )
    }

    fn get_datetime_fields(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)> {
        self.get_fields_at_offset(
            self.datetime_fields_offset,
            key_field_name,
            attributes,
            &SearchFieldConfig::date_from_json,
        )
    }

    pub fn get_key_field(&self) -> Option<SearchFieldName> {
        let key_field_name = self.get_str(self.key_field_offset, "".to_string());
        if key_field_name.is_empty() {
            return None;
        }
        Some(SearchFieldName(key_field_name))
    }

    fn get_key_field_config(
        &self,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
    ) -> Option<(SearchFieldName, SearchFieldConfig, SearchFieldType)> {
        let key_field_name = self.get_key_field()?;
        let key_field_type = attributes.get(&key_field_name)?;
        let key_field_config = match key_field_type {
            SearchFieldType::I64 | SearchFieldType::U64 | SearchFieldType::F64 => {
                SearchFieldConfig::Numeric {
                    indexed: true,
                    fast: true,
                    stored: false,
                    column: None,
                }
            }
            SearchFieldType::Text => SearchFieldConfig::Text {
                indexed: true,
                fast: true,
                stored: false,
                fieldnorms: false,
                tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::raw()),
                record: IndexRecordOption::Basic,
                normalizer: SearchNormalizer::Raw,
                column: None,
            },
            SearchFieldType::Uuid => SearchFieldConfig::default_uuid(),
            SearchFieldType::Json => SearchFieldConfig::Json {
                indexed: true,
                fast: true,
                stored: false,
                fieldnorms: false,
                expand_dots: false,
                tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
                record: IndexRecordOption::Basic,
                normalizer: SearchNormalizer::Raw,
                column: None,
            },
            SearchFieldType::Range => SearchFieldConfig::Range {
                stored: false,
                column: None,
            },
            SearchFieldType::Bool => SearchFieldConfig::Boolean {
                indexed: true,
                fast: true,
                stored: false,
                column: None,
            },
            SearchFieldType::Date => SearchFieldConfig::Date {
                indexed: true,
                fast: true,
                stored: false,
                column: None,
            },
        };

        Some((key_field_name, key_field_config, *key_field_type))
    }

    fn get_ctid_field_config(
        &self,
    ) -> (SearchFieldName, SearchFieldConfig, Option<SearchFieldType>) {
        (
            SearchFieldName("ctid".into()),
            SearchFieldConfig::Numeric {
                indexed: true,
                fast: true,
                stored: false,
                column: None,
            },
            Some(SearchFieldType::U64),
        )
    }

    pub unsafe fn get_all_fields(
        &self,
        indexrel: &PgRelation,
    ) -> impl Iterator<Item = (SearchFieldName, SearchFieldConfig, SearchFieldType)> {
        let heaprel = indexrel
            .heap_relation()
            .expect("index relation should have a heap relation");
        let tupdesc = heaprel.tuple_desc();

        let index_info = unsafe { pg_sys::BuildIndexInfo(indexrel.as_ptr()) };

        let mut attributes: FxHashMap<SearchFieldName, SearchFieldType> = FxHashMap::default();
        let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg((*index_info).ii_Expressions) };
        let mut expressions_iter = expressions.iter_ptr();

        for i in 0..(*index_info).ii_NumIndexAttrs {
            let heap_attno = (*index_info).ii_IndexAttrNumbers[i as usize];
            let (att_readable, att_name, atttypid) = if heap_attno == 0 {
                // Is an expression.
                let Some(expression) = expressions_iter.next() else {
                    panic!("Expected expression for index attribute {i}.");
                };
                let node = expression.cast();

                let expression_str = unsafe {
                    let pg_cstr = pg_sys::deparse_expression(
                        node,
                        pg_sys::deparse_context_for(heaprel.name().as_pg_cstr(), heaprel.oid()),
                        false,
                        false,
                    );
                    let expression_str = core::ffi::CStr::from_ptr(pg_cstr)
                        .to_str()
                        .expect("Invalid UTF8 in result of deparse_expression.")
                        .to_owned();

                    pg_sys::pfree(pg_cstr.cast());
                    expression_str
                };
                (
                    format!("expression '{expression_str}'"),
                    expression_str,
                    pg_sys::exprType(node),
                )
            } else {
                // Is a field.
                let att = tupdesc
                    .get((heap_attno - 1) as usize)
                    .expect("attribute should exist");
                (
                    format!("column '{}'", att.name()),
                    att.name().to_owned(),
                    att.type_oid().value(),
                )
            };

            let array_type = pg_sys::get_element_type(atttypid);
            let base_oid = PgOid::from(if array_type != pg_sys::InvalidOid {
                array_type
            } else {
                atttypid
            });
            let field_type = SearchFieldType::try_from(&base_oid).unwrap_or_else(|err| {
                panic!("cannot index {att_readable} with type {base_oid:?}: {err}",)
            });

            attributes.insert(SearchFieldName(att_name), field_type);
        }

        let (key_field_name, key_field_config, key_field_type) = self
            .get_key_field_config(&attributes)
            .expect("key_field WITH option should be configured");

        let mut configured = self
            .get_text_fields(&key_field_name.0, &attributes)
            .into_iter()
            .chain(self.get_numeric_fields(&key_field_name.0, &attributes))
            .chain(self.get_boolean_fields(&key_field_name.0, &attributes))
            .chain(self.get_json_fields(&key_field_name.0, &attributes))
            .chain(self.get_range_fields(&key_field_name.0, &attributes))
            .chain(self.get_datetime_fields(&key_field_name.0, &attributes))
            .chain(std::iter::once(self.get_ctid_field_config()))
            .map(|(field_name, field_config, field_type)| {
                (
                    field_name.clone(),
                    (
                        field_config,
                        field_type.unwrap_or_else(|| {
                            panic!("field type should have been set for `{field_name}`")
                        }),
                    ),
                )
            })
            .collect::<FxHashMap<SearchFieldName, (SearchFieldConfig, SearchFieldType)>>();

        // make sure the set of configured fields don't specify a different configuration for the key_field
        // we own this configuration
        if configured.contains_key(&key_field_name) {
            panic!("cannot override BM25 configuration for key_field '{key_field_name}', you must use an aliased field name and 'column' configuration key");
        }
        configured.insert(key_field_name, (key_field_config, key_field_type));

        // look for configured fields that don't directly map to an index attribute
        // these should have a `column` value on their config and that column should match
        // a field in the attribute set
        for (field_name, (field_config, _)) in configured.iter() {
            if !attributes.contains_key(field_name) {
                if let Some(column) = field_config.column() {
                    if !attributes.contains_key(&SearchFieldName(column.into())) {
                        panic!("field '{field_name}' references a column '{column}' which is not in the index definition");
                    }
                }
            }
        }

        // assign default configurations for any fields in the attributes set that didn't have user-specified configs
        for (field_name, field_type) in attributes {
            if let Entry::Vacant(entry) = configured.entry(field_name) {
                entry.insert((field_type.default_config(), field_type));
            }
        }

        configured
            .into_iter()
            .map(|(field_name, (field_config, field_type))| (field_name, field_config, field_type))
    }

    fn validate_fields_and_set_types(
        &self,
        key_field_name: &str,
        attributes: &FxHashMap<SearchFieldName, SearchFieldType>,
        fields: &mut Vec<(SearchFieldName, SearchFieldConfig, Option<SearchFieldType>)>,
    ) {
        for (field_name, field_config, outer_field_type) in fields {
            if key_field_name == field_name.0 {
                panic!("cannot override BM25 configuration for key_field '{key_field_name}', you must use an aliased field name and 'column' configuration key");
            }

            if "ctid" == &field_name.0 {
                panic!("the name `ctid` is reserved by pg_search");
            } else if let Some(field_type) = attributes.get(field_name) {
                if !field_type.is_compatible_with(field_config) {
                    panic!("field type '{field_name}' is not compatible with field config '{field_config:?}'")
                }
                *outer_field_type = Some(*field_type);
            } else if let Some(column) = field_config.column() {
                if let Some(field_type) = attributes.get(&SearchFieldName(column.clone())) {
                    *outer_field_type = Some(*field_type);
                } else {
                    panic!("the column `{column} referenced by the field configuration for '{field_name}' does not exist")
                }
            }
        }
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
}
