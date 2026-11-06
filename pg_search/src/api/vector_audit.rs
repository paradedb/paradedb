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

//! Router audit export: the inputs `benchmarks` needs to rank one IVF segment's clusters offline.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use pgrx::JsonB;
use pgrx::prelude::*;
use serde_json::json;
use tantivy::vector::RouterKind;

use crate::api::{CTID_FIELD_NAME, FieldName};
use crate::index::reader::open_vector_field;
use crate::postgres::access::IndexOid;

/// The name the audit reports a persisted router kind under.
fn router_kind_name(code: u8) -> Result<String> {
    [
        RouterKind::Stacked,
        RouterKind::Exact,
        RouterKind::QuantizedStacked,
    ]
    .into_iter()
    .find(|kind| *kind as u8 == code)
    .map(|kind| kind.to_string())
    .with_context(|| format!("unknown router kind {code}"))
}

/// Writes one vector field's router inputs into server directory `dir`:
///
/// - `centroids.f32`: centroid rows in stored (router) order.
/// - `router.bin`: the router slot, its kind byte included.
/// - `sizes.u32`: rows per cluster.
/// - `doc_clusters`: `(ctid u64, cluster u32)` per live document, its primary cluster.
/// - `meta.json`: the shapes and build options these were written under, also returned.
///
/// Numbers are little-endian. The index must hold exactly one IVF segment for `field`.
#[pg_extern]
fn vector_router_export(index: IndexOid, field: String, dir: String) -> Result<JsonB> {
    // SAFETY: `superuser` reads the session's current user and has no preconditions.
    ensure!(
        unsafe { pg_sys::superuser() },
        "vector_router_export writes server files and requires a superuser"
    );
    let index = index.read();
    let (search_reader, vector_field, _) = open_vector_field(&index, &field)?;
    let mut exports = Vec::new();
    for segment_reader in search_reader.segment_readers() {
        let vector_index = segment_reader.vector_index(vector_field)?;
        if let Some(export) = vector_index.router_audit_export()? {
            exports.push((segment_reader, export));
        }
    }
    let [(segment_reader, export)] = exports.as_slice() else {
        bail!(
            "vector_router_export needs one IVF segment, found {}",
            exports.len()
        );
    };

    let dir = Path::new(&dir);
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    fs::write(dir.join("centroids.f32"), export.centroid_rows.as_slice())?;
    fs::write(dir.join("router.bin"), export.router_slot.as_slice())?;
    let mut sizes = BufWriter::new(fs::File::create(dir.join("sizes.u32"))?);
    for size in &export.cluster_sizes {
        sizes.write_all(&size.to_le_bytes())?;
    }
    sizes.flush()?;

    let ctids = segment_reader.fast_fields().u64(CTID_FIELD_NAME)?;
    let alive = segment_reader.alive_bitset();
    let mut docs = BufWriter::new(fs::File::create(dir.join("doc_clusters"))?);
    let mut live_docs = 0u64;
    for &(doc, cluster) in &export.doc_clusters {
        if alive.is_some_and(|alive| !alive.is_alive(doc)) {
            continue;
        }
        let ctid = ctids
            .first(doc)
            .with_context(|| format!("document {doc} has no ctid"))?;
        docs.write_all(&ctid.to_le_bytes())?;
        docs.write_all(&cluster.to_le_bytes())?;
        live_docs += 1;
    }
    docs.flush()?;

    let options = index.options();
    let meta = json!({
        "dim": export.dim,
        "metric": export.metric,
        "clusters": export.cluster_sizes.len(),
        "docs": live_docs,
        "router_kind": router_kind_name(export.router_slot.as_slice()[0])?,
        "max_leaf_size": options.max_leaf_size(),
        "training_sample_ratio": options.training_sample_ratio(),
        "quantization": options
            .field_config_or_default(&FieldName::from(field.as_str()))
            .quantization_layers(),
        "spill": options.spill(),
        "segment_id": segment_reader.segment_id().short_uuid_string(),
    });
    fs::write(dir.join("meta.json"), serde_json::to_vec_pretty(&meta)?)?;
    Ok(JsonB(meta))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::fs;

    use pgrx::prelude::*;
    use tantivy::vector::router_audit::stored_router_kind;

    use super::*;
    use crate::api::vector_test_support::vector_metadata_fixture;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::rel::PgSearchRelation;

    fn export_dir() -> String {
        std::env::temp_dir()
            .join(format!("vector_router_export_{}", std::process::id()))
            .display()
            .to_string()
    }

    #[pg_test]
    fn export_covers_every_row_and_opens_as_stored() {
        let indexrel: PgSearchRelation = vector_metadata_fixture();
        let dir = export_dir();
        let meta = Spi::get_one::<JsonB>(&format!(
            "SELECT paradedb.vector_router_export('metadata_vectors_idx', 'vec', '{dir}')"
        ))
        .unwrap()
        .unwrap()
        .0;
        let dir = Path::new(&dir);

        let sizes = fs::read(dir.join("sizes.u32")).unwrap();
        let (sizes, _) = sizes.as_chunks::<4>();
        let sizes: Vec<u32> = sizes.iter().copied().map(u32::from_le_bytes).collect();
        assert_eq!(meta["clusters"].as_u64().unwrap() as usize, sizes.len());
        assert_eq!(sizes.iter().map(|&size| u64::from(size)).sum::<u64>(), 2048);

        let doc_clusters = fs::read(dir.join("doc_clusters")).unwrap();
        assert_eq!(doc_clusters.len(), 2048 * 12);
        assert_eq!(meta["docs"], 2048);
        let mut counts = vec![0u32; sizes.len()];
        for row in doc_clusters.as_chunks::<12>().0 {
            counts[u32::from_le_bytes(row[8..12].try_into().unwrap()) as usize] += 1;
        }
        assert_eq!(counts, sizes);

        let reader = SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap();
        let field = reader.schema().tantivy_schema().get_field("vec").unwrap();
        let info = reader
            .segment_readers()
            .iter()
            .find_map(|segment| segment.vector_index(field).unwrap().info().unwrap())
            .unwrap();
        let kind = stored_router_kind(
            1024,
            serde_json::from_value(meta["metric"].clone()).unwrap(),
            &fs::read(dir.join("router.bin")).unwrap(),
            &fs::read(dir.join("centroids.f32")).unwrap(),
        )
        .unwrap();
        assert_eq!(Some(kind), info.router);
        assert_eq!(meta["router_kind"], kind.to_string());
        fs::remove_dir_all(dir).unwrap();
    }

    #[pg_test(error = "vector_router_export writes server files and requires a superuser")]
    fn export_needs_a_superuser() {
        vector_metadata_fixture();
        Spi::run(
            "CREATE ROLE vector_audit_reader; GRANT USAGE ON SCHEMA paradedb TO vector_audit_reader;
             SET ROLE vector_audit_reader;",
        )
        .unwrap();
        Spi::run(&format!(
            "SELECT paradedb.vector_router_export('metadata_vectors_idx', 'vec', '{}')",
            export_dir()
        ))
        .unwrap();
    }
}
