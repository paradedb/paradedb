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

pub mod directory;
pub mod fast_fields_helper;
pub mod merge_policy;
pub mod reader;
pub mod search;
pub mod writer;

pub use directory::*;
pub use search::*;

use crate::postgres::options::BM25IndexOptions;
use crate::schema::SearchIndexSchema;
use tantivy::columnar::CodecType;
use tantivy::IndexSettings;

/// The [`IndexSettings`] used for every tantivy index pg_search creates.
///
/// `docstore_compress_dedicated_thread` must remain `false`: a dedicated compressor thread
/// receives process-directed signals, and pgrx's background worker signal handlers call into
/// Postgres FFI, which panics off the main thread. Compress inline instead.
pub fn index_settings(
    options: &BM25IndexOptions,
    schema: &tantivy::schema::Schema,
) -> IndexSettings {
    IndexSettings {
        sort_by_field: SearchIndexSchema::build_sort_by_field(&options.sort_by(), schema),
        docstore_compress_dedicated_thread: false,
        codec_types: vec![CodecType::Bitpacked, CodecType::BlockwiseLinearV2],
        ..IndexSettings::default()
    }
}
