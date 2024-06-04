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

use crate::index::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::schema::{SearchDocument, SearchIndexSchema};
use crate::writer::{IndexError, WriterDirectory};
use pgrx::*;
use serde_json::Map;

pub fn get_search_index(index_name: &str) -> &'static mut SearchIndex {
    let directory = WriterDirectory::from_index_name(index_name);
    SearchIndex::from_cache(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"))
}

pub unsafe fn row_to_search_document(
    tupdesc: &PgTupleDesc,
    values: *mut pg_sys::Datum,
    schema: &SearchIndexSchema,
) -> Result<SearchDocument, IndexError> {
    let mut document = schema.new_document();
    for (attno, attribute) in tupdesc.iter().enumerate() {
        let attname = attribute.name().to_string();
        let attribute_type_oid = attribute.type_oid();

        // If we can't lookup the attribute name in the field_lookup parameter,
        // it means that this field is not part of the index. We should skip it.
        let search_field = if let Some(index_field) = schema.get_search_field(&attname.into()) {
            index_field
        } else {
            continue;
        };

        let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
        let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
            (PgOid::from(array_type), true)
        } else {
            (attribute_type_oid, false)
        };

        let datum = *values.add(attno);

        if is_array {
            for value in TantivyValue::try_from_datum_array(datum, base_oid).unwrap() {
                document.insert(search_field.id, value.tantivy_schema_value());
            }
        } else {
            document.insert(
                search_field.id,
                TantivyValue::try_from_datum(datum, base_oid)?.tantivy_schema_value(),
            );
        }
    }
    pgrx::info!("document: {:?}", document);
    Ok(document)
}
