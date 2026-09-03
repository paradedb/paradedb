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
//! A concurrent build cannot route during its scan, so it runs this once over the segments it
//! wrote, by a tree planned over them, after its workers are done.

use anyhow::{Result, bail};
use tantivy::index::{Index, Segment, SegmentId, SegmentMeta, SegmentReader};
use tantivy::indexer::merger::IndexMerger;
use tantivy::schema::{FieldType, Schema};
use tantivy::{BitSet, DocId};

use pgrx::pg_sys;

use crate::api::FieldName;
use crate::index::fast_fields_helper::FFType;
use crate::index::kdtree::{KdTree, Point};
use crate::index::mvcc::MvccSatisfies;
use crate::index::stats;
use crate::index::writer::index::{SearchIndexMerger, swap_segment_metas};
use crate::postgres::merge::garbage_collect_index;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::schema::SearchFieldType;
use crate::vector::clusterer::set_ivf_clusterer;

/// How many documents the routing sample reads. The tree needs the shape of the distribution,
/// not every value, and the fast fields are read again to route.
const MAX_SAMPLE_DOCS: usize = 30_000;

/// The route of a document that no partition takes, because a delete removed it. It also
/// bounds how many partitions a route can name.
const NO_ROUTE: u16 = u16::MAX;

/// Merges `segment_ids` into one segment per partition of `tree`, and returns the segments
/// it wrote.
pub(crate) fn demux_merge(
    indexrel: &PgSearchRelation,
    segment_ids: &[SegmentId],
    tree: &KdTree,
) -> Result<Vec<SegmentMeta>> {
    if indexrel.options().partition_by().is_empty() {
        bail!("demux_merge requires an index with `partition_by`");
    }
    if segment_ids.is_empty() {
        return Ok(Vec::new());
    }
    if tree.partition_count() >= NO_ROUTE as usize {
        bail!(
            "a demux merge cannot address {} partitions",
            tree.partition_count()
        );
    }

    let schema = indexrel.schema()?;
    let dims = tree.dims();

    let directory = MvccSatisfies::Mergeable.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let sources = sources(&index, segment_ids)?;
    let readers = sources
        .iter()
        .map(SegmentReader::open)
        .collect::<tantivy::Result<Vec<_>>>()?;

    let field_types = dims
        .iter()
        .map(|dim| schema.search_field(dim).map(|field| field.field_type()))
        .collect::<Vec<_>>();

    let routes = routes(&readers, dims, &field_types, tree);

    let mut written = Vec::new();
    for partition in 0..tree.partition_count() {
        pgrx::check_for_interrupts!();
        let masks = masks(&readers, &routes, partition);
        // A partition no document took would write a segment holding nothing.
        if masks.is_none() {
            continue;
        }
        written.push(write_partition(
            indexrel,
            &directory,
            segment_ids,
            partition,
            tree,
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

    swap_segment_metas(
        &index,
        sources.iter().map(|s| s.meta().clone()).collect(),
        written.to_vec(),
    )?;
    Ok(written)
}

/// Whether a demux can read and stamp one dimension: a fast column in raw order.
fn dim_routable(schema: &Schema, dim: &FieldName) -> bool {
    let Ok(field) = schema.get_field(dim.as_ref()) else {
        return false;
    };
    let fast = match schema.get_field_entry(field).field_type() {
        FieldType::Str(options) => options.get_fast_field_tokenizer_name().is_some(),
        FieldType::U64(options) | FieldType::I64(options) | FieldType::F64(options) => {
            options.is_fast()
        }
        FieldType::Bool(options) => options.is_fast(),
        FieldType::Date(options) => options.is_fast(),
        FieldType::Bytes(options) => options.is_fast(),
        _ => false,
    };
    fast && stats::logical_bounds_hold(schema, field)
}

/// Plans the tree a rewrite routes by, over a sample of `segment_ids`.
fn plan_tree(
    indexrel: &PgSearchRelation,
    segment_ids: &[SegmentId],
    target_partitions: usize,
) -> Result<KdTree> {
    let schema = indexrel.schema()?;
    let dims = routable_dims(schema.tantivy_schema(), &indexrel.options().partition_by())?;
    let field_types = dims
        .iter()
        .map(|dim| schema.search_field(dim).map(|field| field.field_type()))
        .collect::<Vec<_>>();
    let directory = MvccSatisfies::Mergeable.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let sources = sources(&index, segment_ids)?;
    let readers = sources
        .iter()
        .map(SegmentReader::open)
        .collect::<tantivy::Result<Vec<_>>>()?;
    let points = sample(&readers, &dims, &field_types);
    Ok(KdTree::from_sample(dims, points, target_partitions))
}

/// The `partition_by` dimensions, checked: every one must have a fast column in raw order,
/// because a demux reads the fast columns and stamps boxes only where the raw order holds
/// (see [`stats::logical_bounds_hold`]). The build could cut on any single-valued field, but
/// a dimension a merge cannot route would leave every later row unrouted forever, so the
/// build and the demux take the same set.
///
/// The build refuses an index that fails this, and `ALTER INDEX` refuses to change
/// `partition_by`, so only an index from before those checks can fail it here, and a
/// concurrent rebuild of one warns instead of routing.
pub(crate) fn routable_dims(schema: &Schema, dims: &[FieldName]) -> Result<Vec<FieldName>> {
    for dim in dims {
        if !dim_routable(schema, dim) {
            bail!(
                "`{dim}` cannot be used in `partition_by` because it does not have a fast column in raw order"
            );
        }
    }
    Ok(dims.to_vec())
}

/// Rewrites every mergeable segment of `indexrel` into one segment per partition, and returns
/// the segments it wrote.
///
/// A build routes the rows it scans; this routes what the index already holds, reading only the
/// segments. A concurrent build calls it once its workers are done, because routing during its
/// scan could break the correspondence between its registered snapshot and the rows it indexes.
pub(crate) unsafe fn route_index(
    indexrel: &PgSearchRelation,
    target_partitions: usize,
) -> Result<Vec<SegmentMeta>> {
    let index_schema = indexrel.schema()?;
    if routable_dims(
        index_schema.tantivy_schema(),
        &indexrel.options().partition_by(),
    )?
    .is_empty()
    {
        return Ok(Vec::new());
    }
    drop(index_schema);
    let metadata = MetaPage::open(indexrel);
    // Hold both locks for the whole rewrite: `ambulkdelete` blocks on the cleanup lock until
    // it can see the segments this leaves behind, and no other backend may merge the sources
    // out from under it.
    let cleanup_lock = metadata.cleanup_lock_shared();
    let merge_lock = metadata.acquire_merge_lock();

    let mut busy = metadata.vacuum_list().read_list();
    busy.extend(merge_lock.merge_list().list_segment_ids());

    // The merger's pins keep the sources readable until the rewrite commits.
    let merger = SearchIndexMerger::open(indexrel, MvccSatisfies::Mergeable)?;
    let segment_ids = merger
        .all_entries()
        .into_iter()
        .filter(|(segment_id, entry)| !busy.contains(segment_id) && entry.is_mergeable(indexrel))
        .map(|(segment_id, _)| segment_id)
        .collect::<Vec<_>>();

    let current_xid = pg_sys::GetCurrentFullTransactionId();
    let next_xid = pg_sys::ReadNextFullTransactionId();
    let mut merge_list = merge_lock.merge_list();
    let merge_entry = merge_list.add_segment_ids(segment_ids.iter(), current_xid)?;
    let written = plan_tree(indexrel, &segment_ids, target_partitions)
        .and_then(|tree| demux_merge(indexrel, &segment_ids, &tree));
    merge_list.remove_entry(merge_entry)?;
    drop(merge_lock);
    drop(merger);

    garbage_collect_index(indexrel, current_xid, next_xid);
    drop(cleanup_lock);
    written
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

/// The point `doc` routes on. A document with no value routes as `NULL`, which the tree sends
/// to the first partition; a frozen mutable segment can hold such documents even under a
/// `NOT NULL` column, because it materializes an unfetchable ctid as an empty document.
fn point(ffs: &[FFType], field_types: &[Option<SearchFieldType>], doc: DocId) -> Point {
    ffs.iter()
        .zip(field_types)
        .map(|(ff, field_type)| ff.value_or_null(doc, *field_type).0)
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

/// The partition of every document, per segment. A deleted document takes [`NO_ROUTE`]. Two
/// bytes per document bound what a route holds in memory, however large the candidate is.
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
    indexrel: &PgSearchRelation,
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
    if indexrel.schema()?.has_vector_field() {
        set_ivf_clusterer(&mut index, indexrel.options());
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
    /// can route. A full rewrite routes what the index already holds.
    #[pg_test]
    fn a_rewrite_routes_an_index_filled_by_inserts() {
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

        let written = unsafe { super::route_index(&indexrel, 4) }.unwrap();
        assert_eq!(written.len(), 4, "one segment per partition");
        drop(indexrel);

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

    /// A merge carries the deletes of its sources, so a rewrite must not resurrect a row that a
    /// `DELETE` removed.
    #[pg_test]
    fn a_rewrite_keeps_deleted_rows_deleted() {
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

        let indexrel = open_index("demux_deleted_idx");
        unsafe { super::route_index(&indexrel, 4) }.unwrap();
        drop(indexrel);

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

    /// A document with no value in the partition column routes as `NULL`, into the first
    /// partition, instead of failing the merge.
    #[pg_test]
    fn a_rewrite_routes_null_partition_values() {
        Spi::run(
            r#"
            CREATE TABLE demux_nulls (id BIGSERIAL PRIMARY KEY, tenant TEXT, name TEXT);
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE INDEX demux_nulls_idx ON demux_nulls USING bm25 (id, tenant, name)
                WITH (key_field = 'id', partition_by = 'tenant', target_segment_count = 4,
                      text_fields = '{"tenant": {"tokenizer": {"type": "keyword"}, "fast": true, "normalizer": "raw"}}');
            INSERT INTO demux_nulls (tenant, name)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE 't' || (i * 7919) % 100 END,
                   'lorem ipsum ' || i
            FROM generate_series(1, 4000) i;
            "#,
        )
        .unwrap();

        let indexrel = open_index("demux_nulls_idx");
        let written = unsafe { super::route_index(&indexrel, 4) }.unwrap();
        assert_eq!(written.len(), 4, "one segment per partition");
        drop(indexrel);

        let indexrel = open_index("demux_nulls_idx");
        persisted_split_points(&indexrel, "tenant")
            .unwrap()
            .expect("the rewrite routed every segment");
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM demux_nulls WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap(),
            4000,
            "the rewrite keeps the rows with no tenant"
        );
    }

    /// A key a demux cannot read holds no values to route on, so the build refuses it up
    /// front instead of leaving an index that no merge can ever heal.
    #[pg_test(
        error = "`tenant_id` cannot be used in `partition_by` because it does not have a fast column in raw order"
    )]
    fn a_non_fast_partition_key_is_rejected() {
        Spi::run(
            r#"
            CREATE TABLE demux_slow (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            CREATE INDEX demux_slow_idx ON demux_slow USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": false}}');
            "#,
        )
        .unwrap();
    }

    /// Every `partition_by` dimension must be one a demux can read back, so a key that
    /// mixes a routable dimension with a plain text one is refused as a whole.
    #[pg_test(
        error = "`name` cannot be used in `partition_by` because it does not have a fast column in raw order"
    )]
    fn a_partly_unroutable_key_is_rejected() {
        Spi::run(
            r#"
            CREATE TABLE demux_subset (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            CREATE INDEX demux_subset_idx ON demux_subset USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id, name', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            "#,
        )
        .unwrap();
    }

    /// Nothing reroutes a built index when its `partition_by` changes, so the change is
    /// refused outright.
    #[pg_test(
        error = "`partition_by` cannot be changed by `ALTER INDEX` because the segments keep the layout it was built with; recreate the index to change it"
    )]
    fn altering_partition_by_is_rejected() {
        Spi::run(
            r#"
            CREATE TABLE demux_altered (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            CREATE INDEX demux_altered_idx ON demux_altered USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            ALTER INDEX demux_altered_idx SET (partition_by = 'tenant_id');
            "#,
        )
        .unwrap();
    }

    /// Clearing `partition_by` is the way out: an empty value reads as "not partitioned"
    /// everywhere, so no segment layout is betrayed and writes keep working.
    #[pg_test]
    fn clearing_partition_by_is_allowed() {
        Spi::run(
            r#"
            CREATE TABLE demux_cleared (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            SET paradedb.global_mutable_segment_rows = 0;
            CREATE INDEX demux_cleared_idx ON demux_cleared USING bm25 (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      numeric_fields = '{"tenant_id": {"fast": true}}');
            INSERT INTO demux_cleared (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i FROM generate_series(1, 2000) i;
            ALTER INDEX demux_cleared_idx RESET (partition_by);
            INSERT INTO demux_cleared (tenant_id, name) VALUES (7, 'late row');
            "#,
        )
        .unwrap();
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM demux_cleared WHERE id @@@ pdb.all();")
                .unwrap()
                .unwrap(),
            2001
        );
    }
}
