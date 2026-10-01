// Copyright (c) 2023-2026 ParadeDB, Inc.
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

pub mod index;
pub mod io_stats;
pub mod scorer;
pub mod segment_component;
pub mod sort_by_range;

use anyhow::{Context, Result, bail};
use tantivy::schema::{Field, FieldType, VectorOptions};

use self::index::SearchIndexReader;
use crate::index::mvcc::MvccSatisfies;
use crate::postgres::rel::PgSearchRelation;

/// Opens a visible vector field and validates its segment headers before reading vector data.
pub(crate) fn open_vector_field(
    index: &PgSearchRelation,
    field: &str,
) -> Result<(SearchIndexReader, Field, VectorOptions)> {
    let reader = SearchIndexReader::empty(index, MvccSatisfies::Snapshot)?;
    let schema = reader.schema().tantivy_schema();
    let vector_field = schema
        .get_field(field)
        .with_context(|| format!("field {field:?} is absent from the index schema"))?;
    let FieldType::Vector(options) = schema.get_field_entry(vector_field).field_type() else {
        bail!("field {field:?} is not a vector field");
    };
    let options = options.clone();
    reader.validate_vector_segments()?;
    Ok((reader, vector_field, options))
}
