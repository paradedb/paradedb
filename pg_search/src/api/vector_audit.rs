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

use anyhow::{Context, Result, ensure};
use pgrx::JsonB;
use pgrx::prelude::*;
use serde_json::{Value, json};
use tantivy::SegmentReader;
use tantivy::vector::RouterKind;
use tantivy::vector::router_audit::RouterAuditExport;

use crate::api::{CTID_FIELD_NAME, FieldName};
use crate::index::reader::open_vector_field;
use crate::index::stats::SegmentStats;
use crate::postgres::access::IndexOid;
use crate::postgres::rel::PgSearchRelation;

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

/// Writes the files every segment's router needs into `dir`.
fn write_router_files(dir: &Path, export: &RouterAuditExport) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    fs::write(dir.join("centroids.f32"), export.centroid_rows.as_slice())?;
    fs::write(dir.join("router.bin"), export.router_slot.as_slice())?;
    let mut sizes = BufWriter::new(fs::File::create(dir.join("sizes.u32"))?);
    for size in &export.cluster_sizes {
        sizes.write_all(&size.to_le_bytes())?;
    }
    sizes.flush()?;
    Ok(())
}

/// The `(ctid, cluster)` of each live document of `segment_reader`, its primary cluster.
fn live_doc_clusters(
    segment_reader: &SegmentReader,
    export: &RouterAuditExport,
) -> Result<Vec<(u64, u32)>> {
    let ctids = segment_reader.fast_fields().u64(CTID_FIELD_NAME)?;
    let alive = segment_reader.alive_bitset();
    export
        .doc_clusters
        .iter()
        .filter(|&&(doc, _)| alive.is_none_or(|alive| alive.is_alive(doc)))
        .map(|&(doc, cluster)| {
            let ctid = ctids
                .first(doc)
                .with_context(|| format!("document {doc} has no ctid"))?;
            Ok((ctid, cluster))
        })
        .collect()
}

/// What `meta.json` records for the index whichever way its segments are laid out.
fn index_meta(index: &PgSearchRelation, field: &str, export: &RouterAuditExport) -> Result<Value> {
    let options = index.options();
    Ok(json!({
        "dim": export.dim,
        "metric": export.metric,
        "router_kind": router_kind_name(export.router_slot.as_slice()[0])?,
        "max_leaf_size": options.max_leaf_size(),
        "training_sample_ratio": options.training_sample_ratio(),
        "quantization": options
            .field_config_or_default(&FieldName::from(field))
            .quantization_layers(),
        "spill": options.spill(),
    }))
}

/// Writes one vector field's router inputs into server directory `dir`:
///
/// - `centroids.f32`: centroid rows in stored (router) order.
/// - `router.bin`: the router slot, its kind byte included.
/// - `sizes.u32`: rows per cluster.
/// - `doc_clusters`: `(ctid u64, cluster u32)` per live document, its primary cluster.
/// - `meta.json`: the shapes and build options these were written under, also returned.
///
/// Numbers are little-endian. With several IVF segments the first three files move to one
/// `segment-<n>` directory each, with that segment's `meta.json` and, when it has one, its
/// partition centroid as `partition.f32`; `doc_clusters` stays at the top and gains the segment,
/// `(ctid u64, segment u32, cluster u32)`.
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
    ensure!(
        !exports.is_empty(),
        "vector_router_export needs an IVF segment"
    );
    let dir = Path::new(&dir);

    if let [(segment_reader, export)] = exports.as_slice() {
        write_router_files(dir, export)?;
        let live = live_doc_clusters(segment_reader, export)?;
        let mut docs = BufWriter::new(fs::File::create(dir.join("doc_clusters"))?);
        for (ctid, cluster) in &live {
            docs.write_all(&ctid.to_le_bytes())?;
            docs.write_all(&cluster.to_le_bytes())?;
        }
        docs.flush()?;
        let mut meta = index_meta(&index, &field, export)?;
        meta["clusters"] = json!(export.cluster_sizes.len());
        meta["docs"] = json!(live.len());
        meta["segment_id"] = json!(segment_reader.segment_id().short_uuid_string());
        fs::write(dir.join("meta.json"), serde_json::to_vec_pretty(&meta)?)?;
        return Ok(JsonB(meta));
    }

    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut docs = BufWriter::new(fs::File::create(dir.join("doc_clusters"))?);
    let mut live_docs = 0usize;
    let mut clusters = 0usize;
    let mut segments = Vec::with_capacity(exports.len());
    for (segment, (segment_reader, export)) in exports.iter().enumerate() {
        let segment_dir = dir.join(format!("segment-{segment}"));
        write_router_files(&segment_dir, export)?;
        let partition = SegmentStats::of_reader(segment_reader)?
            .map(|stats| stats.vector_partition(vector_field))
            .transpose()?
            .flatten();
        if let Some(centroid) = &partition {
            let bytes: Vec<u8> = centroid
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect();
            fs::write(segment_dir.join("partition.f32"), bytes)?;
        }
        let live = live_doc_clusters(segment_reader, export)?;
        for (ctid, cluster) in &live {
            docs.write_all(&ctid.to_le_bytes())?;
            docs.write_all(&(segment as u32).to_le_bytes())?;
            docs.write_all(&cluster.to_le_bytes())?;
        }
        let meta = json!({
            "segment_id": segment_reader.segment_id().short_uuid_string(),
            "clusters": export.cluster_sizes.len(),
            "docs": live.len(),
            "router_kind": router_kind_name(export.router_slot.as_slice()[0])?,
            "partition_centroid": partition.is_some(),
        });
        fs::write(
            segment_dir.join("meta.json"),
            serde_json::to_vec_pretty(&meta)?,
        )?;
        live_docs += live.len();
        clusters += export.cluster_sizes.len();
        segments.push(meta);
    }
    docs.flush()?;
    let mut meta = index_meta(&index, &field, &exports[0].1)?;
    meta["clusters"] = json!(clusters);
    meta["docs"] = json!(live_docs);
    meta["partition_by"] = json!(
        index
            .options()
            .partition_by()
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
    );
    meta["segments"] = json!(segments);
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

    /// Two partitions: one directory per segment with its centroid, and a doc map naming both.
    #[pg_test]
    fn export_writes_one_directory_per_segment() {
        Spi::run(
            "CREATE EXTENSION IF NOT EXISTS vector;
             SET paradedb.vector_clustering_threshold = 64;
             CREATE TABLE partitioned_vectors(id int PRIMARY KEY, vec vector(64));
             INSERT INTO partitioned_vectors SELECT g, ARRAY(
                 SELECT ((CASE WHEN g % 2 = 0 THEN -10 ELSE 10 END) + (g + i) % 17)::real
                 FROM generate_series(1, 64) i)::vector FROM generate_series(1, 4000) g;
             CREATE INDEX partitioned_vectors_idx ON partitioned_vectors
                 USING paradedb(id, vec vector_l2_ops)
                 WITH (quantization = '[1,1]', target_segment_count = 2, partition_by = 'vec');",
        )
        .unwrap();
        let dir = format!("{}_partitioned", export_dir());
        let meta = Spi::get_one::<JsonB>(&format!(
            "SELECT paradedb.vector_router_export('partitioned_vectors_idx', 'vec', '{dir}')"
        ))
        .unwrap()
        .unwrap()
        .0;
        let dir = Path::new(&dir);
        let segments = meta["segments"].as_array().unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(meta["partition_by"], json!(["vec"]));
        assert_eq!(meta["docs"], 4000);

        let doc_clusters = fs::read(dir.join("doc_clusters")).unwrap();
        assert_eq!(doc_clusters.len(), 4000 * 16);
        let mut sums = vec![vec![0u32; 0]; 2];
        for (segment, stored) in segments.iter().enumerate() {
            assert_eq!(stored["partition_centroid"], true);
            let segment_dir = dir.join(format!("segment-{segment}"));
            assert_eq!(
                fs::read(segment_dir.join("partition.f32")).unwrap().len(),
                64 * 4
            );
            let sizes = fs::read(segment_dir.join("sizes.u32")).unwrap();
            sums[segment] = sizes
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(u32::from_le_bytes)
                .collect();
            assert_eq!(
                sums[segment].len() as u64,
                stored["clusters"].as_u64().unwrap()
            );
        }
        let mut counts: Vec<Vec<u32>> = sums.iter().map(|sizes| vec![0; sizes.len()]).collect();
        for row in doc_clusters.as_chunks::<16>().0 {
            let segment = u32::from_le_bytes(row[8..12].try_into().unwrap()) as usize;
            let cluster = u32::from_le_bytes(row[12..16].try_into().unwrap()) as usize;
            counts[segment][cluster] += 1;
        }
        assert_eq!(counts, sums);
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
