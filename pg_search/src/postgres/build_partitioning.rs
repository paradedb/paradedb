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

//! Leader-side boundary generation for a `partition_by` index build.
//!
//! Before the parallel workers launch, the leader samples the heap the same way `ANALYZE` does
//! (a random subset of blocks, then a reservoir over their rows) and builds one
//! [`PartitionTree`] from the sampled `partition_by` values. Sampling the heap rather than
//! reading `pg_statistic` keeps the boundaries correct for tables that were never analyzed and
//! for expression fields, and gives the tree the joint distribution it needs to split on more
//! than one column.

use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::time::Instant;

use anyhow::{Context, bail};
use pgrx::{check_for_interrupts, pg_sys};

use crate::api::FieldName;
use crate::api::tokenizers::definitions::pdb::DatumWithType;
use crate::api::tokenizers::{type_is_alias, type_is_tokenizer};
use crate::index::partition_tree::{PartitionTree, SampleRow};
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{
    collect_composites_for_unpacking, get_field_value, scalar_datum_to_tantivy_value,
};
use crate::schema::{CategorizedFieldData, SearchFieldType};

/// A fixed seed makes a rebuild over an unchanged heap reproduce the same boundaries.
const SAMPLE_SEED: u32 = 0x5EED_1D0C;

/// The `partition_by` fields of an index, in declaration order, with what is needed to pull
/// their values out of a formed index tuple.
struct PartitionDimension {
    field_name: FieldName,
    field_type: SearchFieldType,
    categorized: CategorizedFieldData,
}

/// Computes the global partition boundaries for a `CREATE INDEX` over `heaprel`, or `None`
/// when the index has no `partition_by` or the build targets a single partition.
pub(super) fn plan_partition_tree(
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    target_partitions: usize,
) -> anyhow::Result<Option<PartitionTree>> {
    let partition_by = indexrel.options().partition_by();
    if partition_by.is_empty() || target_partitions <= 1 {
        return Ok(None);
    }

    let dimensions = partition_dimensions(indexrel, &partition_by)?;
    let started = Instant::now();
    let target_rows = sample_target_rows();
    let rows = unsafe { sample_heap(heaprel, indexrel, &dimensions, target_rows)? };
    let nsampled = rows.len();

    let tree = PartitionTree::build(partition_by, rows, target_partitions);
    pgrx::debug1!(
        "build_index: {} partition boundaries over {nsampled} sampled rows in {:?}\n{tree}",
        tree.num_partitions(),
        started.elapsed()
    );
    Ok(Some(tree))
}

/// Same sample size as `ANALYZE`, so the cost of boundary generation is bounded by a setting
/// the DBA already tunes.
fn sample_target_rows() -> usize {
    let statistics_target = unsafe { pg_sys::default_statistics_target }.max(1) as usize;
    300 * statistics_target
}

fn partition_dimensions(
    indexrel: &PgSearchRelation,
    partition_by: &[FieldName],
) -> anyhow::Result<Vec<PartitionDimension>> {
    let schema = indexrel.schema()?;
    let categorized_fields = schema.categorized_fields();
    partition_by
        .iter()
        .map(|field_name| {
            let (search_field, categorized) = categorized_fields
                .iter()
                .find(|(search_field, _)| search_field.field_name() == field_name)
                .with_context(|| format!("`partition_by` field `{field_name}` does not exist"))?;
            let field_type = search_field.field_type();
            if categorized.is_array
                || categorized.is_json
                || matches!(field_type, SearchFieldType::Vector(..))
            {
                bail!("`partition_by` field `{field_name}` must be a scalar field");
            }
            Ok(PartitionDimension {
                field_name: field_name.clone(),
                field_type,
                categorized: categorized.clone(),
            })
        })
        .collect()
}

/// Reads a random subset of `heaprel`'s blocks and keeps a uniform reservoir of at most
/// `target_rows` rows, each reduced to its `partition_by` values.
unsafe fn sample_heap(
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    dimensions: &[PartitionDimension],
    target_rows: usize,
) -> anyhow::Result<Vec<SampleRow>> {
    let nblocks =
        pg_sys::RelationGetNumberOfBlocksInFork(heaprel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM);
    if nblocks == 0 {
        return Ok(vec![]);
    }

    let snapshot = pg_sys::RegisterSnapshot(pg_sys::GetTransactionSnapshot());
    // No `SO_ALLOW_SYNC`: `heap_setscanlimits` needs the start block under our control.
    let flags = pg_sys::ScanOptions::SO_TYPE_SEQSCAN
        | pg_sys::ScanOptions::SO_ALLOW_STRAT
        | pg_sys::ScanOptions::SO_ALLOW_PAGEMODE
        | pg_sys::ScanOptions::SO_TEMP_SNAPSHOT;
    let scan = pg_sys::heap_beginscan(heaprel.as_ptr(), snapshot, 0, null_mut(), null_mut(), flags);
    let slot = pg_sys::table_slot_create(heaprel.as_ptr(), null_mut());
    let estate = pg_sys::CreateExecutorState();
    let econtext = pg_sys::MakePerTupleExprContext(estate);
    (*econtext).ecxt_scantuple = slot;
    let index_info = indexrel.index_info();

    let mut block_sampler = MaybeUninit::<pg_sys::BlockSamplerData>::zeroed();
    pg_sys::BlockSampler_Init(
        block_sampler.as_mut_ptr(),
        nblocks,
        target_rows.min(i32::MAX as usize) as i32,
        SAMPLE_SEED,
    );
    let block_sampler = block_sampler.as_mut_ptr();
    let mut reservoir = Reservoir::new(target_rows, SAMPLE_SEED);

    let mut values = [pg_sys::Datum::null(); pg_sys::INDEX_MAX_KEYS as usize];
    let mut isnull = [false; pg_sys::INDEX_MAX_KEYS as usize];

    let mut scan_started = false;
    let result = (|| -> anyhow::Result<()> {
        while pg_sys::BlockSampler_HasMore(block_sampler) {
            check_for_interrupts!();
            let block = pg_sys::BlockSampler_Next(block_sampler);
            if scan_started {
                pg_sys::heap_rescan(scan, null_mut(), false, false, false, false);
            }
            scan_started = true;
            pg_sys::heap_setscanlimits(scan, block, 1);

            while pg_sys::heap_getnextslot(scan, pg_sys::ScanDirection::ForwardScanDirection, slot)
            {
                pg_sys::FormIndexDatum(
                    index_info,
                    slot,
                    estate,
                    values.as_mut_ptr(),
                    isnull.as_mut_ptr(),
                );
                let row = partition_values(dimensions, values.as_mut_ptr(), isnull.as_mut_ptr())?;
                reservoir.offer(row);
                pg_sys::MemoryContextReset((*econtext).ecxt_per_tuple_memory);
            }
        }
        Ok(())
    })();

    pg_sys::heap_endscan(scan);
    pg_sys::ExecDropSingleTupleTableSlot(slot);
    pg_sys::FreeExecutorState(estate);

    result?;
    Ok(reservoir.into_rows())
}

/// Extracts the `partition_by` values of one formed index tuple, converted exactly as the
/// index writer would store them so the workers' routing and these boundaries agree.
unsafe fn partition_values(
    dimensions: &[PartitionDimension],
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
) -> anyhow::Result<SampleRow> {
    let unpacked_composites = CompositeSlotValues::from_composites(
        collect_composites_for_unpacking(dimensions.iter().map(|d| &d.categorized), values, isnull),
    );
    dimensions
        .iter()
        .map(|dimension| {
            let categorized = &dimension.categorized;
            let (datum, is_null) = get_field_value(
                &categorized.source,
                categorized.attno,
                values,
                isnull,
                &unpacked_composites,
            );
            if is_null {
                return Ok(PdbOwnedValue::Null);
            }
            let pg_type = categorized.pg_type.value();
            let datum = if type_is_alias(pg_type) || type_is_tokenizer(pg_type) {
                DatumWithType::get_underlying_type(datum).0
            } else {
                datum
            };
            let value =
                scalar_datum_to_tantivy_value(datum, dimension.field_type, categorized.base_oid)
                    .with_context(|| {
                        format!(
                            "could not read `partition_by` field `{}`",
                            dimension.field_name
                        )
                    })?;
            Ok(value.0)
        })
        .collect()
}

/// Uniform reservoir sample (Algorithm R) over the rows of the sampled blocks. Rows within a
/// block are correlated (they were inserted together), so keeping every row of fewer blocks
/// would skew the quantiles.
struct Reservoir {
    capacity: usize,
    seen: u64,
    rows: Vec<SampleRow>,
    prng: pg_sys::pg_prng_state,
}

impl Reservoir {
    fn new(capacity: usize, seed: u32) -> Self {
        let mut prng = MaybeUninit::<pg_sys::pg_prng_state>::zeroed();
        unsafe { pg_sys::pg_prng_seed(prng.as_mut_ptr(), seed as u64) };
        Self {
            capacity,
            seen: 0,
            rows: Vec::with_capacity(capacity),
            prng: unsafe { prng.assume_init() },
        }
    }

    fn offer(&mut self, row: SampleRow) {
        self.seen += 1;
        if self.rows.len() < self.capacity {
            self.rows.push(row);
            return;
        }
        let slot = unsafe { pg_sys::pg_prng_uint64_range(&mut self.prng, 0, self.seen - 1) };
        if (slot as usize) < self.capacity {
            self.rows[slot as usize] = row;
        }
    }

    fn into_rows(self) -> Vec<SampleRow> {
        self.rows
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;
    use std::ops::Bound;

    fn relations(table: &str, index: &str) -> (PgSearchRelation, PgSearchRelation) {
        let heap_oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{table}'::regclass::oid"))
            .unwrap()
            .unwrap();
        let index_oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{index}'::regclass::oid"))
            .unwrap()
            .unwrap();
        (
            PgSearchRelation::open(heap_oid),
            PgSearchRelation::open(index_oid),
        )
    }

    fn i64_bound(bound: &Bound<PdbOwnedValue>) -> Option<i64> {
        match bound {
            Bound::Included(PdbOwnedValue::I64(v)) | Bound::Excluded(PdbOwnedValue::I64(v)) => {
                Some(*v)
            }
            Bound::Unbounded => None,
            other => panic!("unexpected bound {other:?}"),
        }
    }

    /// A table small enough that every block is sampled and no row is evicted from the
    /// reservoir yields exact quantiles, so the boundaries are fully determined by the data.
    #[pg_test]
    fn plan_partition_tree_splits_evenly() {
        Spi::run(
            r#"
            CREATE TABLE ptree (id SERIAL PRIMARY KEY, owner_id INT, body TEXT);
            INSERT INTO ptree (owner_id, body)
            SELECT (i * 7919) % 1000, 'row ' || i FROM generate_series(1, 4000) i;
            CREATE INDEX ptree_idx ON ptree USING bm25 (id, owner_id, body)
            WITH (key_field = 'id', partition_by = 'id,owner_id');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = relations("ptree", "ptree_idx");
        let tree = plan_partition_tree(&heaprel, &indexrel, 8)
            .unwrap()
            .expect("partition_by is set, so a tree is planned");
        assert_eq!(tree.num_partitions(), 8);
        assert_eq!(
            tree.dimensions()
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>(),
            vec!["id", "owner_id"]
        );

        // The root split is on `id` at its median; the second level splits `owner_id` and the
        // third refines `id`, so the median shows up as the outer `id` bound of the four
        // partitions adjacent to it.
        let bounds = |p: usize| tree.partition_bounds(p).unwrap();
        assert_eq!(i64_bound(&bounds(1)[0].upper), Some(2001));
        assert_eq!(i64_bound(&bounds(3)[0].upper), Some(2001));
        assert_eq!(i64_bound(&bounds(4)[0].lower), Some(2001));
        assert_eq!(i64_bound(&bounds(6)[0].lower), Some(2001));

        // Every partition holds about a quarter of the table on `id` (the second `id` split
        // is the median of an `owner_id` half, so it is a few rows off) and exactly half of
        // it on `owner_id`.
        for partition in 0..8 {
            let bounds = tree.partition_bounds(partition).unwrap();
            let id_span = i64_bound(&bounds[0].upper).unwrap_or(4001)
                - i64_bound(&bounds[0].lower).unwrap_or(1);
            assert!(
                (980..=1020).contains(&id_span),
                "partition {partition}: {bounds:?}"
            );
            let owner_span = i64_bound(&bounds[1].upper).unwrap_or(1000)
                - i64_bound(&bounds[1].lower).unwrap_or(0);
            assert_eq!(owner_span, 500, "partition {partition}: {bounds:?}");
        }

        // Routing a real row lands in the partition whose bounds contain it.
        let partition = tree.route(&[PdbOwnedValue::I64(2500), PdbOwnedValue::I64(10)]);
        let bounds = tree.partition_bounds(partition).unwrap();
        assert!(i64_bound(&bounds[0].lower).unwrap() <= 2500);
        assert!(i64_bound(&bounds[0].upper).unwrap_or(i64::MAX) > 2500);
        assert!(i64_bound(&bounds[1].lower).unwrap_or(i64::MIN) <= 10);
        assert!(i64_bound(&bounds[1].upper).unwrap() > 10);
    }

    #[pg_test]
    fn plan_partition_tree_handles_expressions_nulls_and_text() {
        Spi::run(
            r#"
            CREATE TABLE ptree_expr (id SERIAL PRIMARY KEY, name TEXT, created_at TIMESTAMP);
            INSERT INTO ptree_expr (name, created_at)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE chr(97 + (i % 26)) || i END,
                   '2020-01-01'::timestamp + (i || ' minutes')::interval
            FROM generate_series(1, 2600) i;
            CREATE INDEX ptree_expr_idx ON ptree_expr
            USING bm25 (id, (name::pdb.literal), created_at)
            WITH (key_field = 'id', partition_by = 'name,created_at');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = relations("ptree_expr", "ptree_expr_idx");
        let tree = plan_partition_tree(&heaprel, &indexrel, 4)
            .unwrap()
            .unwrap();
        assert_eq!(tree.num_partitions(), 4);

        // NULL names route to the first partition, and text split points are real strings.
        assert_eq!(tree.route(&[PdbOwnedValue::Null, PdbOwnedValue::Null]), 0);
        let mid = tree.partition_bounds(2).unwrap();
        assert!(matches!(
            mid[0].lower,
            Bound::Included(PdbOwnedValue::Str(_))
        ));
        assert!(
            matches!(mid[1].lower, Bound::Included(PdbOwnedValue::Date(_)))
                || matches!(mid[1].upper, Bound::Excluded(PdbOwnedValue::Date(_)))
        );
    }

    #[pg_test]
    fn plan_partition_tree_skips_unpartitioned_and_single_target() {
        Spi::run(
            r#"
            CREATE TABLE ptree_plain (id SERIAL PRIMARY KEY, body TEXT);
            INSERT INTO ptree_plain (body) SELECT 'row ' || i FROM generate_series(1, 100) i;
            CREATE INDEX ptree_plain_idx ON ptree_plain USING bm25 (id, body)
            WITH (key_field = 'id');
            CREATE TABLE ptree_part (id SERIAL PRIMARY KEY, body TEXT);
            INSERT INTO ptree_part (body) SELECT 'row ' || i FROM generate_series(1, 100) i;
            CREATE INDEX ptree_part_idx ON ptree_part USING bm25 (id, body)
            WITH (key_field = 'id', partition_by = 'id');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = relations("ptree_plain", "ptree_plain_idx");
        assert!(
            plan_partition_tree(&heaprel, &indexrel, 8)
                .unwrap()
                .is_none()
        );

        let (heaprel, indexrel) = relations("ptree_part", "ptree_part_idx");
        assert!(
            plan_partition_tree(&heaprel, &indexrel, 1)
                .unwrap()
                .is_none()
        );
        let tree = plan_partition_tree(&heaprel, &indexrel, 4)
            .unwrap()
            .unwrap();
        assert_eq!(tree.num_partitions(), 4);
    }
}
