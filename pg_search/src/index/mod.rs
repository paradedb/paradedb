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

pub(crate) mod ctid_map;
pub mod directory;
pub mod fast_fields_helper;
pub mod kdtree;
pub mod merge_policy;
pub mod reader;
pub mod search;
pub(crate) mod segment_pruning;
pub mod stats;
pub mod writer;

pub use directory::*;
pub use search::*;

use crate::api::FieldName;
use crate::index::mvcc::MVCCDirectory;
use crate::postgres::options::BM25IndexOptions;
use crate::schema::SearchIndexSchema;
use std::sync::Arc;

use anyhow::Result;
use pgrx::pg_sys;
use tantivy::columnar::CodecType;
use tantivy::schema::FieldType;
use tantivy::vector::VectorQuantizationConfig;
use tantivy::vector::clustering::SuperKMeansClusterer;
use tantivy::{Index, IndexSettings};

/// Opens the tantivy index behind `directory` with the index's IVF clusterer installed.
pub fn open_index(directory: MVCCDirectory) -> tantivy::Result<Index> {
    let indexrel = directory.indexrel().clone();
    let mut index = Index::open(directory)?;
    set_ivf_clusterer(&mut index, indexrel.options());
    Ok(index)
}

/// Installs tantivy's SuperKMeans clusterer from the index options and the session's
/// `maintenance_work_mem` (CREATE INDEX / VACUUM merge budget).
///
/// Tantivy doesn't persist the clusterer, so every open sets it.
pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    // `maintenance_work_mem` is in kB; train/assign batch sizing wants bytes.
    let memory_limit = (unsafe { pg_sys::maintenance_work_mem as usize }).saturating_mul(1024);
    let clusterer = SuperKMeansClusterer::new()
        .with_max_leaf_size(options.max_leaf_size())
        .with_training_sample_ratio(options.training_sample_ratio())
        .with_memory_limit(memory_limit.max(1));
    index.set_ivf_clusterer(Arc::new(clusterer));
}

/// Builds the Tantivy settings for a ParadeDB index.
///
/// # Errors
///
/// Returns an error when a quantized field is invalid.
pub fn index_settings(
    options: &BM25IndexOptions,
    schema: &tantivy::schema::Schema,
) -> Result<IndexSettings> {
    let mut vector_quantization = Vec::new();
    for (_, field_entry) in schema.fields() {
        let FieldType::Vector(vector_options) = field_entry.field_type() else {
            continue;
        };
        let field_name = field_entry.name();
        let field_config = options.field_config_or_default(&FieldName::from(field_name));
        let Some(bits) = field_config.quantization_layers() else {
            continue;
        };
        vector_quantization.push(VectorQuantizationConfig::with_random_seeds(
            field_name.to_string(),
            vector_options,
            &bits,
        )?);
    }

    Ok(IndexSettings {
        sort_by_field: SearchIndexSchema::build_sort_by_field(&options.sort_by(), schema),
        docstore_compress_dedicated_thread: false,
        codec_types: vec![CodecType::Bitpacked, CodecType::BlockwiseLinearV2],
        vector_clustering_threshold: crate::gucs::vector_clustering_threshold(),
        vector_centroid_quantization: crate::gucs::vector_centroid_quantization(),
        vector_quantization,
        ..IndexSettings::default()
    })
}
