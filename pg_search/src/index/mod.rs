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
pub mod kdtree;
pub mod merge_policy;
pub mod reader;
pub mod search;
pub mod stats;
pub mod writer;

pub use directory::*;
pub use search::*;

use crate::api::FieldName;
use crate::postgres::options::BM25IndexOptions;
use crate::schema::SearchIndexSchema;
use anyhow::{Context, Result};
use rand::{TryRng, rngs::SysRng};
use tantivy::IndexSettings;
use tantivy::columnar::CodecType;
use tantivy::schema::FieldType;
use tantivy::vector::{VectorQuantizationConfig, VectorQuantizationLayer};

/// Builds the Tantivy settings for a ParadeDB index.
///
/// # Errors
///
/// Returns an error when a quantized field is invalid or seed generation fails.
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
        let mut os_rng = SysRng;
        let layers = bits
            .into_iter()
            .map(|bits| {
                let seed = os_rng
                    .try_next_u64()
                    .context("failed to obtain an operating-system random quantization seed")?;
                Ok(VectorQuantizationLayer { bits, seed })
            })
            .collect::<Result<Vec<_>>>()?;
        vector_quantization.push(VectorQuantizationConfig::materialize(
            field_name.to_string(),
            vector_options,
            layers,
        )?);
    }

    Ok(IndexSettings {
        sort_by_field: SearchIndexSchema::build_sort_by_field(&options.sort_by(), schema),
        docstore_compress_dedicated_thread: false,
        codec_types: vec![CodecType::Bitpacked, CodecType::BlockwiseLinearV2],
        vector_clustering_threshold: crate::gucs::vector_clustering_threshold(),
        vector_quantization,
        ..IndexSettings::default()
    })
}
