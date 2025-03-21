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

use crate::postgres::options::SearchIndexCreateOptions;
use crate::schema::{SearchFieldConfig, SearchFieldName, SearchFieldType};
use pgrx::PgRelation;

type Fields = Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>;
type KeyFieldIndex = usize;
pub unsafe fn get_fields(index_relation: &PgRelation) -> (Fields, KeyFieldIndex) {
    let options = SearchIndexCreateOptions::from_relation(index_relation);
    let fields = options.get_all_fields(index_relation).collect::<Vec<_>>();
    let key_field = options.get_key_field().expect("key_field is required");

    let key_field_index = fields
        .iter()
        .position(|(name, _, _)| name == &key_field)
        .expect("key field not found in columns"); // key field is already validated by now.

    (fields, key_field_index)
}
