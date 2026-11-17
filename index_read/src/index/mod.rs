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

pub mod ctid_map;
pub mod directory;
pub mod fast_fields_helper;
pub mod kdtree;
pub mod reader;
pub use storage::index::{index_settings, search, set_ivf_clusterer};
pub mod segment_pruning;
pub mod stats;
pub(crate) mod synonym_cache;
pub mod synonym_index;
pub mod writer;

pub use directory::*;
pub use search::*;

use crate::index::mvcc::MVCCDirectory;
use tantivy::Index;

/// Opens the tantivy index behind `directory` with the index's IVF clusterer installed.
pub fn open_index(directory: MVCCDirectory) -> tantivy::Result<Index> {
    let indexrel = directory.indexrel().clone();
    let mut index = Index::open(directory)?;
    let memory_limit =
        (unsafe { pgrx::pg_sys::maintenance_work_mem as usize }).saturating_mul(1024);
    set_ivf_clusterer(&mut index, indexrel.options(), memory_limit);
    Ok(index)
}
