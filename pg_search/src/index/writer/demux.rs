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

//! A merge that routes its documents instead of concatenating them.
//!
//! An ordinary merge takes N segments and writes one. This one takes N segments, plans
//! partition boundaries over the values they hold, and writes one segment per partition, each
//! stamped with its box. The rows it writes are the rows it read, so it keeps the visibility
//! its sources recorded, unlike a rebuild from the heap.
//!
//! That is what a `partition_by` index needs to keep its layout after the build: rows arriving
//! through `INSERT` land in unrouted segments, and only a build could route them until now.

use anyhow::{Result, bail};
use tantivy::index::{Index, IndexMeta, Segment, SegmentId, SegmentMeta, SegmentReader};
use tantivy::indexer::merger::IndexMerger;
use tantivy::{BitSet, Directory, DocId};

use crate::api::FieldName;
use crate::api::HashSet;
use crate::index::fast_fields_helper::FFType;
use crate::index::kdtree::{KdTree, Point};
use crate::index::mvcc::MvccSatisfies;
use crate::index::stats;
use crate::index::stats::SegmentStats;
use crate::postgres::rel::PgSearchRelation;
use crate::schema::SearchFieldType;

/// How many documents the routing sample reads. The tree needs the shape of the distribution,
/// not every value, and the fast fields are read again to route.
const MAX_SAMPLE_DOCS: usize = 30_000;

/// The route of a document that no partition takes, because a delete removed it. It also
/// bounds how many partitions a route can name.
const NO_ROUTE: u16 = u16::MAX;

/// Merges `segment_ids` into one segment per partition, and returns the segments it wrote.
///
/// The boundaries come from the values these segments hold, so they describe this merge only.
/// A later merge plans its own, and the split points a reader collects are the union of every
/// box in the index.
pub(crate) fn demux_merge(
    indexrel: &PgSearchRelation,
    segment_ids: &[SegmentId],
    target_partitions: usize,
) -> Result<Vec<SegmentMeta>> {
    let dims = indexrel.options().partition_by();
    if dims.is_empty() {
        bail!("demux_merge requires an index with `partition_by`");
    }
    if segment_ids.is_empty() {
        return Ok(Vec::new());
    }

    let directory = MvccSatisfies::Mergeable.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let sources = sources(&index, segment_ids)?;
    let readers = sources
        .iter()
        .map(SegmentReader::open)
        .collect::<tantivy::Result<Vec<_>>>()?;

    let schema = indexrel.schema()?;
    let field_types = dims
        .iter()
        .map(|dim| schema.search_field(dim).map(|field| field.field_type()))
        .collect::<Vec<_>>();

    let tree = KdTree::from_sample(
        dims.clone(),
        sample(&readers, &dims, &field_types),
        target_partitions,
    );
    if tree.partition_count() >= NO_ROUTE as usize {
        bail!(
            "a demux merge cannot address {} partitions",
            tree.partition_count()
        );
    }
    let routes = routes(&readers, &dims, &field_types, &tree);

    let mut written = Vec::new();
    for partition in 0..tree.partition_count() {
        let masks = masks(&readers, &routes, partition);
        // A partition no document took would write a segment holding nothing.
        if masks.is_none() {
            continue;
        }
        written.push(write_partition(
            &directory,
            segment_ids,
            partition,
            &tree,
            masks.unwrap(),
        )?);
    }

    // Every live document of the sources has to reach exactly one output. Losing one here
    // would be silent, so nothing is published until the counts agree.
    let live: u64 = readers.iter().map(|r| r.num_docs() as u64).sum();
    let routed: u64 = written.iter().map(|meta| meta.max_doc() as u64).sum();
    if live != routed {
        bail!("a demux merge routed {routed} of {live} documents");
    }

    commit(&directory, &index, &sources, &written)?;
    Ok(written)
}

/// The mergeable segments of `indexrel` that carry no box, so nothing routed them.
///
/// A build stamps every segment it writes. A segment without a box came in afterwards, through
/// an `INSERT` or a merge that could not prove one.
pub(crate) fn unrouted_segments(indexrel: &PgSearchRelation) -> Result<HashSet<SegmentId>> {
    let dims = indexrel.options().partition_by();
    let Some(dim) = dims.first() else {
        return Ok(HashSet::default());
    };
    let directory = MvccSatisfies::Mergeable.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let Ok(field) = index.schema().get_field(dim.as_ref()) else {
        return Ok(HashSet::default());
    };
    let mut unrouted = HashSet::default();
    for segment in index.searchable_segments()? {
        let boxed = SegmentStats::of_segment(&segment)?
            .map(|stats| stats.logical(field))
            .transpose()?
            .flatten()
            .is_some();
        if !boxed {
            unrouted.insert(segment.id());
        }
    }
    Ok(unrouted)
}

/// The segments of `index` named by `segment_ids`, in that order.
fn sources(index: &Index, segment_ids: &[SegmentId]) -> Result<Vec<Segment>> {
    let mut by_id = index
        .searchable_segments()?
        .into_iter()
        .map(|segment| (segment.id(), segment))
        .collect::<std::collections::HashMap<_, _>>();
    segment_ids
        .iter()
        .map(|id| {
            by_id
                .remove(id)
                .ok_or_else(|| anyhow::anyhow!("segment {id} is not mergeable"))
        })
        .collect()
}

/// The point `doc` routes on.
fn point(ffs: &[FFType], field_types: &[Option<SearchFieldType>], doc: DocId) -> Point {
    ffs.iter()
        .zip(field_types)
        .map(|(ff, field_type)| ff.value(doc, *field_type).0)
        .collect()
}

fn fast_fields(reader: &SegmentReader, dims: &[FieldName]) -> Vec<FFType> {
    dims.iter()
        .map(|dim| FFType::new(reader.fast_fields(), dim.as_ref()))
        .collect()
}

/// A stride sample of the values the segments hold, for the boundaries to be planned over.
fn sample(
    readers: &[SegmentReader],
    dims: &[FieldName],
    field_types: &[Option<SearchFieldType>],
) -> Vec<Point> {
    let total: usize = readers
        .iter()
        .map(|reader| reader.num_docs() as usize)
        .sum();
    let stride = total.div_ceil(MAX_SAMPLE_DOCS).max(1);
    let mut sample = Vec::with_capacity(total.min(MAX_SAMPLE_DOCS));
    let mut seen = 0usize;
    for reader in readers {
        let ffs = fast_fields(reader, dims);
        for doc in reader.doc_ids_alive() {
            if seen.is_multiple_of(stride) {
                sample.push(point(&ffs, field_types, doc));
            }
            seen += 1;
        }
    }
    sample
}

/// The partition of every document, per segment. A deleted document takes [`NO_ROUTE`].
fn routes(
    readers: &[SegmentReader],
    dims: &[FieldName],
    field_types: &[Option<SearchFieldType>],
    tree: &KdTree,
) -> Vec<Vec<u16>> {
    readers
        .iter()
        .map(|reader| {
            let ffs = fast_fields(reader, dims);
            let mut routes = vec![NO_ROUTE; reader.max_doc() as usize];
            for doc in reader.doc_ids_alive() {
                routes[doc as usize] = tree.route(&point(&ffs, field_types, doc)) as u16;
            }
            routes
        })
        .collect()
}

/// One alive set per segment holding the documents `partition` takes, or `None` when it takes
/// nothing at all.
fn masks(
    readers: &[SegmentReader],
    routes: &[Vec<u16>],
    partition: usize,
) -> Option<Vec<Option<tantivy::fastfield::AliveBitSet>>> {
    let partition = partition as u16;
    let mut any = false;
    let masks = readers
        .iter()
        .zip(routes)
        .map(|(reader, routes)| {
            let mut bitset = BitSet::with_max_value(reader.max_doc());
            for (doc, route) in routes.iter().enumerate() {
                if *route == partition {
                    bitset.insert(doc as DocId);
                    any = true;
                }
            }
            let mut bytes = Vec::new();
            tantivy::fastfield::write_alive_bitset(&bitset, &mut bytes)
                .expect("writing an alive set to memory cannot fail");
            Some(tantivy::fastfield::AliveBitSet::open(
                tantivy::directory::OwnedBytes::new(bytes),
            ))
        })
        .collect::<Vec<_>>();
    any.then_some(masks)
}

/// Merges the documents `masks` selects into one segment carrying `partition`'s box.
fn write_partition(
    directory: &crate::index::directory::mvcc::MVCCDirectory,
    segment_ids: &[SegmentId],
    partition: usize,
    tree: &KdTree,
    masks: Vec<Option<tantivy::fastfield::AliveBitSet>>,
) -> Result<SegmentMeta> {
    // The plugin set travels with the segments, so the box has to be registered on the index
    // they are opened from, before they are opened.
    let mut index = Index::open(directory.clone())?;
    match stats::partition_box(tree, partition) {
        Some(logical) => stats::register_with_bounds(&mut index, logical),
        None => stats::register(&mut index),
    }
    let sources = sources(&index, segment_ids)?;

    let merger = IndexMerger::open_with_custom_alive_set(
        index.schema(),
        index.settings().clone(),
        &sources,
        masks,
        Box::new(|| false),
        true,
    )?;
    let target = index.new_segment();
    let num_docs = merger.write(&target)?;
    Ok(index.new_segment_meta(target.id(), num_docs))
}

/// Replaces the source segments with the ones the merge wrote, in one atomic list update.
fn commit(
    directory: &crate::index::directory::mvcc::MVCCDirectory,
    index: &Index,
    sources: &[Segment],
    written: &[SegmentMeta],
) -> Result<()> {
    let meta = |segments: Vec<SegmentMeta>| IndexMeta {
        index_settings: index.settings().clone(),
        persisted_custom_extensions: index
            .custom_plugins()
            .iter()
            .flat_map(|plugin| plugin.extensions().iter().map(|ext| ext.to_string()))
            .collect(),
        segments,
        schema: index.schema(),
        opstamp: 0,
        payload: None,
    };
    // `save_metas` reads the created and deleted ids out of the difference between the two
    // lists, so these hold the merge's inputs and outputs rather than the whole index.
    directory.save_metas(
        &meta(written.to_vec()),
        &meta(sources.iter().map(|s| s.meta().clone()).collect()),
        &mut (),
    )?;
    Ok(())
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    use crate::index::stats::persisted_split_points;
    use crate::postgres::rel::PgSearchRelation;

    fn open_index(index: &str) -> PgSearchRelation {
        let oid = Spi::get_one::<pg_sys::Oid>(&format!(
            "SELECT '{index}'::regclass::oid FROM pg_class WHERE relname = '{index}';"
        ))
        .unwrap()
        .unwrap();
        PgSearchRelation::open(oid)
    }

    /// An index created before its rows holds no boundaries, and a build is the only thing that
    /// can route. `repartition` routes what the index already holds.
    #[pg_test]
    fn repartition_routes_an_index_filled_by_inserts() {
        Spi::run(
            r#"
            CREATE TABLE demux_late (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE INDEX demux_late_idx ON demux_late USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            INSERT INTO demux_late (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i FROM generate_series(1, 4000) i;
            "#,
        )
        .unwrap();

        let indexrel = open_index("demux_late_idx");
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none(),
            "an insert cannot route"
        );
        let before = Spi::get_one::<i64>("SELECT count(*) FROM demux_late WHERE id @@@ pdb.all();")
            .unwrap()
            .unwrap();
        drop(indexrel);

        let written = Spi::get_one::<i64>("SELECT paradedb.repartition('demux_late_idx');")
            .unwrap()
            .unwrap();
        assert_eq!(written, 4, "one segment per partition");

        let indexrel = open_index("demux_late_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("the rewrite routed every segment");
        assert_eq!(split_points.len(), 3);
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM demux_late WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap(),
            before,
            "the rewrite keeps every row"
        );
    }

    /// Nothing routes the rows an `INSERT` adds, so the merge that takes those segments has to.
    /// No explicit call: an ordinary insert-time merge heals the layout.
    #[pg_test]
    fn a_merge_routes_what_no_build_routed() {
        Spi::run(
            r#"
            CREATE TABLE demux_merged (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE INDEX demux_merged_idx ON demux_merged USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            "#,
        )
        .unwrap();
        for batch in 0..5 {
            Spi::run(&format!(
                r#"
                INSERT INTO demux_merged (tenant_id, name)
                SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding here ', 20)
                FROM generate_series({} , {}) i;
                "#,
                batch * 1000 + 1,
                (batch + 1) * 1000
            ))
            .unwrap();
        }

        let indexrel = open_index("demux_merged_idx");
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none(),
            "an insert cannot route"
        );
        drop(indexrel);

        // A layer closes a candidate once its segments fill it by a third over, so a layer of
        // 3.4 times the largest segment takes all five in one. Background layers would hand the
        // merge to a worker that cannot see this transaction's segments.
        let largest: i64 = Spi::get_one(
            "SELECT max(byte_size)::bigint FROM paradedb.index_info('demux_merged_idx');",
        )
        .unwrap()
        .unwrap();
        Spi::run(&format!(
            "ALTER INDEX demux_merged_idx SET (layer_sizes = '{}', background_layer_sizes = '0');",
            largest * 17 / 5
        ))
        .unwrap();
        Spi::run("INSERT INTO demux_merged (tenant_id, name) VALUES (42, 'late row');").unwrap();

        let indexrel = open_index("demux_merged_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("the merge routed the segments it took");
        assert_eq!(split_points.len(), 3);
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM demux_merged WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap(),
            5001,
            "the merge keeps every row"
        );
    }

    /// A merge carries the deletes of its sources, so a rewrite must not resurrect a row that a
    /// `DELETE` removed.
    #[pg_test]
    fn repartition_keeps_deleted_rows_deleted() {
        Spi::run(
            r#"
            CREATE TABLE demux_deleted (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE INDEX demux_deleted_idx ON demux_deleted USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            INSERT INTO demux_deleted (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i FROM generate_series(1, 4000) i;
            DELETE FROM demux_deleted WHERE id % 4 = 0;
            "#,
        )
        .unwrap();

        let before =
            Spi::get_one::<i64>("SELECT count(*) FROM demux_deleted WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap();
        assert_eq!(before, 3000);

        Spi::run("SELECT paradedb.repartition('demux_deleted_idx');").unwrap();

        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM demux_deleted WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap(),
            before
        );
        assert_eq!(
            Spi::get_one::<i64>(
                "SELECT count(*) FROM demux_deleted WHERE id @@@ pdb.all() AND id % 4 = 0;"
            )
            .unwrap()
            .unwrap(),
            0,
            "a deleted row stays deleted"
        );
    }
}
