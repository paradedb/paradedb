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

use crate::index::directory::blocking::BlockingDirectory;
use crate::index::{SearchIndex, SearchIndexError};
use crate::postgres::options::SearchIndexCreateOptions;
use crate::schema::{
    IndexRecordOption, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
};
use pgrx::{pg_sys, PgBox, PgOid, PgRelation};
use std::collections::HashMap;
use tantivy::Index;
use tokenizers::manager::SearchTokenizerFilters;
use tokenizers::{SearchNormalizer, SearchTokenizer};

use crate::postgres::storage::block::METADATA_BLOCKNO;
use crate::postgres::storage::utils::BM25BufferCache;

/// Open the underlying [`SearchIndex`] for the specified Postgres index relation
pub fn open_search_index(
    index_relation: &PgRelation,
) -> anyhow::Result<SearchIndex, SearchIndexError> {
    let index_oid = index_relation.oid();
    let cache = unsafe { BM25BufferCache::open(index_oid) };
    let lock = unsafe { cache.get_buffer(METADATA_BLOCKNO, Some(pgrx::pg_sys::BUFFER_LOCK_SHARE)) };

    let (fields, key_field_index) = unsafe { get_fields(index_relation) };
    let schema = SearchIndexSchema::new(fields, key_field_index)?;
    let tantivy_dir = BlockingDirectory::new(index_oid);
    let mut underlying_index = Index::open(tantivy_dir)?;

    SearchIndex::setup_tokenizers(&mut underlying_index, &schema);

    unsafe { pg_sys::UnlockReleaseBuffer(lock) };

    Ok(SearchIndex {
        schema,
        underlying_index,
        index_oid,
    })
}

type Fields = Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>;
type KeyFieldIndex = usize;
pub unsafe fn get_fields(index_relation: &PgRelation) -> (Fields, KeyFieldIndex) {
    let heap_relation = index_relation
        .heap_relation()
        .expect("index should belong to a heap");
    let rdopts: PgBox<SearchIndexCreateOptions> = if !index_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(index_relation.rd_options as *mut SearchIndexCreateOptions) }
    } else {
        let ops = unsafe { PgBox::<SearchIndexCreateOptions>::alloc0() };
        ops.into_pg_boxed()
    };

    // Create a map from column name to column type. We'll use this to verify that index
    // configurations passed by the user reference the correct types for each column.
    let name_type_map: HashMap<SearchFieldName, SearchFieldType> = heap_relation
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

    for (name, _) in rdopts.get_text_fields() {
        if !matches!(name_type_map.get(&name), Some(SearchFieldType::Text)) {
            panic!("'{name}' cannot be indexed as a text field");
        }
    }

    for (name, _) in rdopts.get_numeric_fields() {
        if !matches!(
            name_type_map.get(&name),
            Some(SearchFieldType::U64 | SearchFieldType::I64 | SearchFieldType::F64)
        ) {
            panic!("'{name}' cannot be indexed as a numeric field");
        }
    }

    for (name, _) in rdopts.get_boolean_fields() {
        if !matches!(name_type_map.get(&name), Some(SearchFieldType::Bool)) {
            panic!("'{name}' cannot be indexed as a boolean field");
        }
    }

    for (name, _) in rdopts.get_json_fields() {
        if !matches!(name_type_map.get(&name), Some(SearchFieldType::Json)) {
            panic!("'{name}' cannot be indexed as a JSON field");
        }
    }

    for (name, _) in rdopts.get_range_fields() {
        if !matches!(name_type_map.get(&name), Some(SearchFieldType::Range)) {
            panic!("'{name}' cannot be indexed as a range field");
        }
    }

    for (name, _) in rdopts.get_datetime_fields() {
        if !matches!(name_type_map.get(&name), Some(SearchFieldType::Date)) {
            panic!("'{name}' cannot be indexed as a datetime field");
        }
    }

    let key_field = rdopts.get_key_field().expect("must specify key field");
    let key_field_type = match name_type_map.get(&key_field) {
        Some(field_type) => field_type,
        None => panic!("key field does not exist"),
    };
    let key_config = match key_field_type {
        SearchFieldType::I64 | SearchFieldType::U64 | SearchFieldType::F64 => {
            SearchFieldConfig::Numeric {
                indexed: true,
                fast: true,
                stored: true,
            }
        }
        SearchFieldType::Text => SearchFieldConfig::Text {
            indexed: true,
            fast: true,
            stored: true,
            fieldnorms: false,
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
        },
        SearchFieldType::Json => SearchFieldConfig::Json {
            indexed: true,
            fast: true,
            stored: true,
            fieldnorms: false,
            expand_dots: false,
            tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::default()),
            record: IndexRecordOption::Basic,
            normalizer: SearchNormalizer::Raw,
        },
        SearchFieldType::Range => SearchFieldConfig::Range { stored: true },
        SearchFieldType::Bool => SearchFieldConfig::Boolean {
            indexed: true,
            fast: true,
            stored: true,
        },
        SearchFieldType::Date => SearchFieldConfig::Date {
            indexed: true,
            fast: true,
            stored: true,
        },
    };

    // Concatenate the separate lists of fields.
    let index_info = unsafe { pg_sys::BuildIndexInfo(index_relation.as_ptr()) };
    let fields: Vec<_> = rdopts
        .get_fields(&heap_relation, index_info)
        .into_iter()
        .filter(|(name, _, _)| name != &key_field) // Process key_field separately.
        .chain(std::iter::once((
            key_field.clone(),
            key_config,
            *key_field_type,
        )))
        // "ctid" is a reserved column name in Postgres, so we don't need to worry about
        // creating a name conflict with a user-named column.
        .chain(std::iter::once((
            "ctid".into(),
            SearchFieldConfig::Ctid,
            SearchFieldType::U64,
        )))
        .collect();

    let key_field_index = fields
        .iter()
        .position(|(name, _, _)| name == &key_field)
        .expect("key field not found in columns"); // key field is already validated by now.
    (fields, key_field_index)
}
