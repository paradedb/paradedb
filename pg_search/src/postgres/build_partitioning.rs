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

//! Global partition boundaries for `CREATE INDEX`.
//!
//! Parallel build workers each see an arbitrary slice of the heap, so boundaries chosen from
//! worker-local data would not line up across workers. The leader instead samples the heap
//! once, before any worker starts, and builds a scalar kd-tree or vector centroids
//! that every worker then routes with.
//!
//! The sample is taken from the heap rather than from `pg_statistic`: a freshly loaded table
//! usually has no statistics yet, expression fields never have any, and per-column histograms
//! only describe marginals, while a recursive split needs the joint distribution of the
//! `partition_by` fields.

use anyhow::{Result, bail};
use std::ptr::addr_of_mut;

use pgrx::itemptr::item_pointer_set_all;
use pgrx::{FromDatum, PgMemoryContexts, check_for_interrupts, pg_sys};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use superkmeans::distance::l2_squared;
use superkmeans::{SuperKMeans, SuperKMeansConfig};
use tantivy::schema::{Field, FieldType, Schema};

use crate::api::FieldName;
use crate::api::version::Version;
use crate::index::kdtree::{KdTree, Point};
use crate::index::stats;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::heap::{ExpressionState, HeapBufferPin};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::BorrowedBuffer;
use crate::postgres::utils::{
    FieldSource, item_pointer_to_u64, resolve_field_value, scalar_datum_to_tantivy_value,
    unwrap_alias_datum,
};
use crate::schema::{CategorizedFieldData, SearchField, SearchFieldType};
use crate::vector::PgVector;
use crate::vector::metric::VectorMetric;

/// Upper bound on the heap blocks read for the sample. Blocks are chosen uniformly at random,
/// so a table physically ordered by a `partition_by` field still yields a spread of positions
/// across its key range.
const MAX_SAMPLE_BLOCKS: u32 = 4096;

/// Upper bound on the rows kept from the sampled blocks, the same size `ANALYZE` uses at the
/// default statistics target. Rows beyond it are reservoir-sampled so the kept set stays
/// uniform over the visited blocks.
const MAX_SAMPLE_ROWS: usize = 30_000;

/// A fixed seed makes a rebuild over an unchanged heap reproduce the same boundaries.
const SAMPLE_SEED: u32 = 0x5EED_1D0C;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) enum BuildPartitioning {
    Range(KdTree),
    Vector(VectorPartitioning),
}

impl BuildPartitioning {
    pub(super) fn dims(&self) -> &[FieldName] {
        match self {
            Self::Range(tree) => tree.dims(),
            Self::Vector(model) => std::slice::from_ref(&model.field),
        }
    }

    pub(super) fn partition_count(&self) -> usize {
        match self {
            Self::Range(tree) => tree.partition_count(),
            Self::Vector(model) => (model.centroids.len() / model.dimensions).max(1),
        }
    }

    pub(super) fn route(&self, point: &PartitionPoint) -> usize {
        match (self, point) {
            (Self::Range(tree), PartitionPoint::Range(point)) => tree.route(point),
            (Self::Vector(model), PartitionPoint::Vector(vector)) => model.route(vector.as_deref()),
            _ => unreachable!("partition projection must match its model"),
        }
    }
}

pub(super) enum PartitionPoint {
    Range(Point),
    Vector(Option<Vec<f32>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct VectorPartitioning {
    field: FieldName,
    dimensions: usize,
    metric: VectorMetric,
    centroids: Vec<f32>,
}

impl VectorPartitioning {
    fn train(
        field: FieldName,
        dimensions: usize,
        metric: VectorMetric,
        sample: Vec<PartitionPoint>,
        target: usize,
    ) -> Result<Self> {
        let mut data = Vec::new();
        let mut distinct = std::collections::HashSet::new();
        for point in sample {
            let PartitionPoint::Vector(vector) = point else {
                unreachable!("vector partition sample must contain vectors")
            };
            let Some(mut vector) = vector else { continue };
            anyhow::ensure!(
                vector.len() == dimensions,
                "partition vector dimension mismatch"
            );
            anyhow::ensure!(
                vector.iter().all(|v| v.is_finite()),
                "partition vector must be finite"
            );
            if metric == VectorMetric::Cosine {
                let norm = vector
                    .iter()
                    .map(|&v| f64::from(v).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if norm == 0.0 {
                    continue;
                }
                for value in &mut vector {
                    *value = (f64::from(*value) / norm) as f32;
                }
            }
            if distinct.len() < target {
                distinct.insert(
                    vector
                        .iter()
                        .map(|v| if *v == 0.0 { 0 } else { v.to_bits() })
                        .collect::<Vec<_>>(),
                );
            }
            data.extend(vector);
        }
        let count = target.min(distinct.len());
        let centroids = if count > 1 {
            let mut clusterer = SuperKMeans::with_config(
                count,
                dimensions,
                SuperKMeansConfig {
                    seed: SAMPLE_SEED as u64,
                    n_threads: 1,
                    angular: metric != VectorMetric::L2,
                    use_blas_only: true,
                    data_already_rotated: true,
                    suppress_warnings: true,
                    ..Default::default()
                },
            );
            let rows = data.len() / dimensions;
            clusterer.train_owned(data, rows)
        } else {
            Vec::new()
        };
        anyhow::ensure!(
            centroids.iter().all(|v| v.is_finite()),
            "vector partition training produced non-finite centroids"
        );
        Ok(Self {
            field,
            dimensions,
            metric,
            centroids,
        })
    }

    fn route(&self, vector: Option<&[f32]>) -> usize {
        let Some(vector) = vector else { return 0 };
        assert_eq!(vector.len(), self.dimensions);
        self.centroids
            .chunks_exact(self.dimensions)
            .enumerate()
            .map(|(partition, centroid)| {
                let distance = match self.metric {
                    VectorMetric::L2 => l2_squared(vector, centroid),
                    // Angular training keeps centroids on the unit sphere; the row norm
                    // is constant across centroids and does not affect its assignment.
                    VectorMetric::Cosine | VectorMetric::InnerProduct => {
                        let mut sums = [0.0; 8];
                        let (rows, row_tail) = vector.as_chunks::<8>();
                        let (centers, center_tail) = centroid.as_chunks::<8>();
                        for (row, center) in rows.iter().zip(centers) {
                            for lane in 0..8 {
                                sums[lane] += row[lane] * center[lane];
                            }
                        }
                        let tail: f32 = row_tail.iter().zip(center_tail).map(|(a, b)| a * b).sum();
                        -(sums.iter().sum::<f32>() + tail)
                    }
                };
                (partition, distance)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(0, |(partition, _)| partition)
    }
}

/// Computes the global partition boundaries for a build of `indexrel`, or `None` when the
/// index does not declare `partition_by`.
///
/// `snapshot` is the one the build scan itself uses: `SnapshotAny` for a plain build, or the
/// registered MVCC snapshot of a `CONCURRENTLY` build. The sample then covers the same rows the
/// workers are about to index. `target_partitions` is the number of leaves to aim for; the tree
/// can come out smaller when the sample lacks the distinct values to cut further (see
/// [`KdTree::from_sample`]).
pub(super) fn plan_partition_boundaries(
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    snapshot: pg_sys::Snapshot,
    target_partitions: usize,
) -> anyhow::Result<Option<BuildPartitioning>> {
    let partition_by = indexrel.options().partition_by();
    if partition_by.is_empty() {
        return Ok(None);
    }
    let vector = match indexrel.options().get_field_type(&partition_by[0]) {
        Some(SearchFieldType::Vector(_, dimensions, metric)) => Some((dimensions, metric)),
        _ => None,
    };
    if target_partitions <= 1 {
        return Ok(Some(match vector {
            Some((dimensions, metric)) => BuildPartitioning::Vector(VectorPartitioning {
                field: partition_by[0].clone(),
                dimensions,
                metric,
                centroids: Vec::new(),
            }),
            None => BuildPartitioning::Range(KdTree::unpartitioned(partition_by)),
        }));
    }

    let sample_rows = vector.map_or(MAX_SAMPLE_ROWS, |(dimensions, _)| {
        let budget = (unsafe { pg_sys::maintenance_work_mem } as usize).saturating_mul(1024);
        (budget
            / dimensions
                .max(1)
                .saturating_mul(std::mem::size_of::<f32>() * 4))
        .clamp(1, MAX_SAMPLE_ROWS)
    });
    let sample = unsafe {
        sample_partition_fields(heaprel, indexrel, snapshot, &partition_by, sample_rows)?
    };
    pgrx::debug1!(
        "plan_partition_boundaries: sampled {} rows for partition_by={partition_by:?}, target_partitions={target_partitions}",
        sample.len()
    );
    if let Some((dimensions, metric)) = vector {
        return Ok(Some(BuildPartitioning::Vector(VectorPartitioning::train(
            partition_by[0].clone(),
            dimensions,
            metric,
            sample,
            target_partitions.max(1),
        )?)));
    }
    let sample = sample
        .into_iter()
        .map(|point| match point {
            PartitionPoint::Range(point) => point,
            PartitionPoint::Vector(_) => {
                unreachable!("range partition sample must contain scalar values")
            }
        })
        .collect();
    Ok(Some(BuildPartitioning::Range(KdTree::from_sample(
        partition_by,
        sample,
        target_partitions,
    ))))
}

/// Whether `dim` has a columnar field at all, whatever order that column is in.
fn dim_fast(schema: &Schema, field: Field) -> bool {
    match schema.get_field_entry(field).field_type() {
        FieldType::Str(options) => options.get_fast_field_tokenizer_name().is_some(),
        FieldType::U64(options) | FieldType::I64(options) | FieldType::F64(options) => {
            options.is_fast()
        }
        FieldType::Bool(options) => options.is_fast(),
        FieldType::Date(options) => options.is_fast(),
        FieldType::Bytes(options) => options.is_fast(),
        _ => false,
    }
}

/// Refuses a dimension with no columnar field. `sort_by` has no column to order a segment
/// by without one, and `partition_by` has none to run its range query against.
pub(crate) fn check_fast_dims(schema: &Schema, dims: &[FieldName], reloption: &str) -> Result<()> {
    for dim in dims {
        let Ok(field) = schema.get_field(dim.as_ref()) else {
            bail!("{reloption} field '{dim}' does not exist in the index schema");
        };
        if reloption == "partition_by"
            && matches!(
                schema.get_field_entry(field).field_type(),
                FieldType::Vector(_)
            )
        {
            if dims.len() != 1 {
                bail!("a vector partition_by field must be the only partition field");
            }
            continue;
        }
        if !dim_fast(schema, field) {
            bail!(
                "{reloption} field '{dim}' must be columnar. Add it to the index with 'columnar=true'"
            );
        }
    }
    Ok(())
}

/// The `dims` whose columnar field orders by a normalized value, so a segment's logical bounds
/// cannot hold for the raw range (see [`stats::logical_bounds_hold`]). What that costs depends on the
/// caller: `partition_by` cuts on the raw value and cannot prune without it, while `sort_by`
/// still lays the segment out and only gives up its bounds.
pub(crate) fn normalized_dims(schema: &Schema, dims: &[FieldName]) -> Vec<FieldName> {
    dims.iter()
        .filter(|dim| {
            schema
                .get_field(dim.as_ref())
                .is_ok_and(|field| !stats::logical_bounds_hold(schema, field))
        })
        .cloned()
        .collect()
}

/// Where a `partition_by` field's value comes from for a heap tuple, plus what it takes to
/// turn that datum into the value the index stores.
#[derive(Clone)]
pub(crate) struct CategorizedPartitionField {
    pub(crate) field: SearchField,
    pub(crate) categorized: CategorizedFieldData,
}

impl CategorizedPartitionField {
    pub(super) unsafe fn datum_to_vector(
        &self,
        datum: pg_sys::Datum,
        is_null: bool,
    ) -> Option<Vec<f32>> {
        if is_null {
            return None;
        }
        let datum = unsafe { unwrap_alias_datum(datum, self.categorized.pg_type) };
        unsafe { PgVector::from_datum(datum, false) }.map(|vector| vector.0)
    }

    pub(crate) unsafe fn datum_to_value(
        &self,
        datum: pg_sys::Datum,
        is_null: bool,
        created_by_version: Option<Version>,
    ) -> anyhow::Result<PdbOwnedValue> {
        if is_null {
            return Ok(PdbOwnedValue::Null);
        }
        let datum = unsafe { unwrap_alias_datum(datum, self.categorized.pg_type) };
        let value = unsafe {
            scalar_datum_to_tantivy_value(
                datum,
                self.field.field_type(),
                self.categorized.base_oid,
                created_by_version,
            )
        }
        .map_err(|e| {
            anyhow::anyhow!(
                "could not project partition_by field `{}`: {e}",
                self.field.field_name()
            )
        })?;
        Ok(value.0)
    }
}

#[derive(Clone)]
pub(crate) enum PartitionField {
    Ctid,
    Categorized(Box<CategorizedPartitionField>),
}

impl PartitionField {
    pub(crate) fn resolve_dims(
        dims: &[FieldName],
        categorized_fields: &[(SearchField, CategorizedFieldData)],
    ) -> anyhow::Result<Vec<Self>> {
        dims.iter()
            .map(|dim| {
                if dim.is_ctid() {
                    Ok(Self::Ctid)
                } else {
                    categorized_fields
                        .iter()
                        .find(|(field, _)| field.field_name() == dim)
                        .cloned()
                        .map(|(field, categorized)| {
                            Self::Categorized(Box::new(CategorizedPartitionField {
                                field,
                                categorized,
                            }))
                        })
                        .ok_or_else(|| anyhow::anyhow!("partition_by field `{dim}` is not indexed"))
                }
            })
            .collect()
    }
}

/// Reads a random subset of `heaprel`'s blocks and projects every row the build will index
/// onto `partition_by`, in that order.
unsafe fn sample_partition_fields(
    heaprel: &PgSearchRelation,
    indexrel: &PgSearchRelation,
    snapshot: pg_sys::Snapshot,
    partition_by: &[FieldName],
    sample_rows: usize,
) -> anyhow::Result<Vec<PartitionPoint>> {
    let schema = indexrel.schema()?;
    let categorized_fields = schema.categorized_fields();
    let fields = PartitionField::resolve_dims(partition_by, &categorized_fields)?;
    drop(categorized_fields);

    let nblocks = unsafe {
        pg_sys::RelationGetNumberOfBlocksInFork(heaprel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
    };
    if nblocks == 0 {
        return Ok(Vec::new());
    }

    // A plain build scans with `SnapshotAny` and lets `HeapTupleSatisfiesVacuum` drop only the
    // dead tuples, so the sample applies the same test; an MVCC snapshot is applied directly.
    let snapshot_is_any =
        unsafe { (*snapshot).snapshot_type == pg_sys::SnapshotType::SNAPSHOT_ANY };
    let oldest_xmin = if snapshot_is_any {
        unsafe { pg_sys::GetOldestNonRemovableTransactionId(heaprel.as_ptr()) }
    } else {
        pg_sys::InvalidTransactionId
    };
    let is_indexed = |tuple: *mut pg_sys::HeapTupleData, buffer: pg_sys::Buffer| unsafe {
        if snapshot_is_any {
            pg_sys::HeapTupleSatisfiesVacuum(tuple, oldest_xmin, buffer)
                != pg_sys::HTSV_Result::HEAPTUPLE_DEAD
        } else {
            pg_sys::HeapTupleSatisfiesVisibility(tuple, snapshot, buffer)
        }
    };

    let needs_expressions = fields.iter().any(|f| match f {
        PartitionField::Ctid => false,
        PartitionField::Categorized(categorized) => {
            !matches!(categorized.categorized.source, FieldSource::Heap { .. })
        }
    });
    let expression_state = needs_expressions.then(|| ExpressionState::new(indexrel));

    let mut sampler = unsafe {
        let mut sampler: pg_sys::BlockSamplerData = std::mem::zeroed();
        pg_sys::BlockSampler_Init(
            addr_of_mut!(sampler),
            nblocks,
            nblocks.min(MAX_SAMPLE_BLOCKS) as i32,
            SAMPLE_SEED,
        );
        sampler
    };

    let mut rng = StdRng::seed_from_u64(SAMPLE_SEED as u64);
    let mut per_block_context = PgMemoryContexts::new("pg_search partition sampling");
    let slot = unsafe {
        pg_sys::MakeTupleTableSlot(heaprel.rd_att, &raw const pg_sys::TTSOpsBufferHeapTuple)
    };
    let natts = unsafe { (*heaprel.rd_att).natts as usize };
    let created_by_version = indexrel.created_by_version();

    let mut sample = Vec::new();
    let mut seen_rows = 0usize;
    let mut indexed_offsets: Vec<pg_sys::OffsetNumber> = Vec::new();
    let mut result: anyhow::Result<()> = Ok(());
    unsafe {
        while pg_sys::BlockSampler_HasMore(addr_of_mut!(sampler)) {
            check_for_interrupts!();
            let blockno = pg_sys::BlockSampler_Next(addr_of_mut!(sampler));

            let buffer = HeapBufferPin::read(heaprel, blockno);
            let buf = buffer.buffer();
            let page = pg_sys::BufferGetPage(buf);

            // Under the content lock, only test visibility (which also sets hint bits) and record
            // the offsets to index. Detoast, expression evaluation, and projection run afterward
            // under the pin alone, so a concurrent writer or a re-entrant index expression never
            // waits on this page's lock across that work.
            indexed_offsets.clear();
            {
                let _content_lock = BorrowedBuffer::from_pg(buf);
                let max_offset = pg_sys::PageGetMaxOffsetNumber(page);
                for offset in pg_sys::FirstOffsetNumber..=max_offset {
                    let item_id = pg_sys::PageGetItemId(page, offset);
                    if (*item_id).lp_flags() != pg_sys::LP_NORMAL {
                        continue;
                    }
                    let mut t_self = pg_sys::ItemPointerData::default();
                    item_pointer_set_all(&mut t_self, blockno, offset);
                    let mut tuple = pg_sys::HeapTupleData {
                        t_len: (*item_id).lp_len(),
                        t_self,
                        t_tableOid: heaprel.oid(),
                        t_data: pg_sys::PageGetItem(page, item_id).cast(),
                    };
                    if is_indexed(addr_of_mut!(tuple), buf) {
                        indexed_offsets.push(offset);
                    }
                }
            }

            // The pin blocks pruning and eviction, so the line pointers and tuple data recorded
            // above stay valid while we read them here.
            let block_result = per_block_context.switch_to(|_| {
                for &offset in &indexed_offsets {
                    // Decide whether this row makes the sample before paying to project it.
                    seen_rows += 1;
                    let target_slot = if sample.len() < sample_rows {
                        None
                    } else {
                        let j = rng.random_range(0..seen_rows);
                        if j >= sample_rows {
                            continue;
                        }
                        Some(j)
                    };

                    let item_id = pg_sys::PageGetItemId(page, offset);
                    let mut t_self = pg_sys::ItemPointerData::default();
                    item_pointer_set_all(&mut t_self, blockno, offset);
                    let mut tuple = pg_sys::HeapTupleData {
                        t_len: (*item_id).lp_len(),
                        t_self,
                        t_tableOid: heaprel.oid(),
                        t_data: pg_sys::PageGetItem(page, item_id).cast(),
                    };
                    pg_sys::ExecStoreBufferHeapTuple(addr_of_mut!(tuple), slot, buf);
                    pg_sys::slot_getallattrs(slot);
                    let values = std::slice::from_raw_parts((*slot).tts_values, natts);
                    let isnull = std::slice::from_raw_parts((*slot).tts_isnull, natts);
                    let expr_results = expression_state
                        .as_ref()
                        .map(|state| state.evaluate(slot))
                        .unwrap_or_default();

                    // The sample uses the tuple's own physical ctid, whereas `build_callback`
                    // routes the HOT root.
                    let ctid_u64 = item_pointer_to_u64(t_self);
                    let point = project_row(
                        &fields,
                        values,
                        isnull,
                        ctid_u64,
                        &expr_results,
                        created_by_version,
                    );
                    // The stored pointer targets this iteration's `tuple`; clear it before that
                    // goes out of scope, on the error path too.
                    pg_sys::ExecClearTuple(slot);
                    let point = point?;

                    match target_slot {
                        None => sample.push(point),
                        Some(j) => sample[j] = point,
                    }
                }
                anyhow::Ok(())
            });
            per_block_context.reset();

            if let Err(e) = block_result {
                result = Err(e);
                break;
            }
            // `buffer` drops here, releasing the pin, on both the normal and the break paths.
        }
        pg_sys::ExecDropSingleTupleTableSlot(slot);
    }

    result?;
    Ok(sample)
}

/// Projects one deformed heap row (plus its evaluated index expressions) onto the sampled
/// fields, converting each datum the same way the index writer does.
unsafe fn project_row(
    fields: &[PartitionField],
    values: &[pg_sys::Datum],
    isnull: &[bool],
    ctid: u64,
    expr_results: &[(pg_sys::Datum, bool)],
    created_by_version: Option<Version>,
) -> anyhow::Result<PartitionPoint> {
    let unpacked_composites = unsafe {
        CompositeSlotValues::from_composites(fields.iter().filter_map(|f| match f {
            PartitionField::Categorized(categorized) => {
                if let FieldSource::CompositeField {
                    expression_idx,
                    composite_type_oid,
                    ..
                } = categorized.categorized.source
                {
                    let (datum, is_null) = expr_results[expression_idx];
                    Some((expression_idx, datum, is_null, composite_type_oid))
                } else {
                    None
                }
            }
            PartitionField::Ctid => None,
        }))
    };

    if let [PartitionField::Categorized(categorized)] = fields
        && matches!(categorized.field.field_type(), SearchFieldType::Vector(..))
    {
        let (datum, is_null) = resolve_field_value(
            &categorized.categorized.source,
            values,
            isnull,
            expr_results,
            &unpacked_composites,
        );
        return Ok(PartitionPoint::Vector(unsafe {
            categorized.datum_to_vector(datum, is_null)
        }));
    }

    fields
        .iter()
        .map(|f| match f {
            PartitionField::Ctid => Ok(PdbOwnedValue::U64(ctid)),
            PartitionField::Categorized(categorized) => {
                let (datum, is_null) = resolve_field_value(
                    &categorized.categorized.source,
                    values,
                    isnull,
                    expr_results,
                    &unpacked_composites,
                );
                unsafe { categorized.datum_to_value(datum, is_null, created_by_version) }
            }
        })
        .collect::<Result<Point>>()
        .map(PartitionPoint::Range)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::datum::DatumWithOid;
    use pgrx::prelude::*;

    #[pg_test]
    fn vector_partition_training_and_transport() {
        for metric in [
            VectorMetric::L2,
            VectorMetric::Cosine,
            VectorMetric::InnerProduct,
        ] {
            let model = VectorPartitioning::train(
                FieldName::from("emb"),
                2,
                metric,
                (0..200)
                    .map(|i| {
                        PartitionPoint::Vector(Some(vec![
                            if i % 2 == 0 { -10.0 } else { 10.0 },
                            (i % 7) as f32 * 0.01,
                        ]))
                    })
                    .collect(),
                2,
            )
            .unwrap();
            assert_eq!(model.centroids.len(), 4);
            assert_ne!(
                model.route(Some(&[-10.0, 0.0])),
                model.route(Some(&[10.0, 0.0]))
            );
            assert_eq!(model.route(None), 0);
            if metric != VectorMetric::L2 {
                assert_eq!(model.route(Some(&[0.0, 0.0])), 0);
            }
            let model = BuildPartitioning::Vector(model);
            let bytes = postcard::to_allocvec(&model).unwrap();
            assert_eq!(
                postcard::from_bytes::<BuildPartitioning>(&bytes).unwrap(),
                model
            );
        }
        for sample in [
            vec![],
            vec![PartitionPoint::Vector(None)],
            vec![PartitionPoint::Vector(Some(vec![0.0, 0.0]))],
        ] {
            let model = VectorPartitioning::train(
                FieldName::from("emb"),
                2,
                VectorMetric::Cosine,
                sample,
                8,
            )
            .unwrap();
            assert_eq!(model.route(Some(&[1.0, 0.0])), 0);
            assert_eq!(BuildPartitioning::Vector(model).partition_count(), 1);
        }
        let model = VectorPartitioning::train(
            FieldName::from("emb"),
            2,
            VectorMetric::L2,
            (0..100)
                .map(|_| PartitionPoint::Vector(Some(vec![1.0, 2.0])))
                .collect(),
            8,
        )
        .unwrap();
        assert_eq!(BuildPartitioning::Vector(model).partition_count(), 1);
    }

    #[pg_test]
    fn vector_partitions_group_neighbors_and_keep_nulls() {
        Spi::run(
            r#"
            CREATE EXTENSION IF NOT EXISTS vector;
            CREATE TABLE vector_partition_src (id BIGINT, region INTEGER, emb vector(2));
            INSERT INTO vector_partition_src
            SELECT i, i % 2, ARRAY[CASE WHEN i % 2 = 0 THEN -10 ELSE 10 END, (i % 7) * 0.01]::vector
            FROM generate_series(1, 4000) i;
            INSERT INTO vector_partition_src VALUES (4001, NULL, NULL), (4002, NULL, '[0,0]');
        "#,
        )
        .unwrap();
        for workers in [0, 2] {
            Spi::run("SET parallel_leader_participation = false").unwrap();
            Spi::run(&format!(
                "SET max_parallel_workers = 8; SET max_parallel_maintenance_workers = {workers};"
            ))
            .unwrap();
            for metric in ["vector_l2_ops", "vector_cosine_ops", "vector_ip_ops"] {
                Spi::run(&format!(
                    r#"
                    CREATE INDEX vector_partition_idx ON vector_partition_src
                    USING paradedb (id, region, emb {metric})
                    WITH (partition_by = 'emb', target_segment_count = 2);
                "#
                ))
                .unwrap();
                assert_eq!(
                    Spi::get_one::<i64>(
                        "SELECT count(*) FROM paradedb.index_info('vector_partition_idx')"
                    )
                    .unwrap(),
                    Some(2)
                );
                assert_eq!(
                    Spi::get_one::<i64>(
                        "SELECT count(*) FROM vector_partition_src WHERE id @@@ pdb.all()"
                    )
                    .unwrap(),
                    Some(4002)
                );
                assert_eq!(Spi::get_one::<bool>(r#"
                    SELECT bool_and(regions = 1) FROM (
                        SELECT unnest(paradedb.find_ctid('vector_partition_idx', ctid)), count(DISTINCT region) AS regions
                        FROM vector_partition_src GROUP BY 1
                    ) partitions
                "#).unwrap(), Some(true));
                Spi::run("DROP INDEX vector_partition_idx").unwrap();
            }
        }
    }

    #[pg_test(error = "a vector partition_by field must be the only partition field")]
    fn vector_partition_rejects_mixed_fields() {
        Spi::run(
            r#"
            CREATE EXTENSION IF NOT EXISTS vector;
            CREATE TABLE mixed_partition_src (id BIGINT, emb vector(2));
            CREATE INDEX ON mixed_partition_src USING paradedb (id, emb)
            WITH (partition_by = 'id, emb');
        "#,
        )
        .unwrap();
    }

    /// The snapshot a plain (non-concurrent) build scans with.
    fn snapshot_any() -> pg_sys::Snapshot {
        &raw mut pg_sys::SnapshotAnyData
    }

    fn open_rels(table: &str, index: &str) -> (PgSearchRelation, PgSearchRelation) {
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

    /// Routes every row of `table` through `tree` and returns the per-partition counts.
    fn route_all(tree: &KdTree, table: &str, columns: &str) -> Vec<usize> {
        let mut counts = vec![0usize; tree.partition_count()];
        Spi::connect(|client| {
            let args: [DatumWithOid; 0] = [];
            let rows = client.select(&format!("SELECT {columns} FROM {table}"), None, &args)?;
            for row in rows {
                let point: Point = (1..=row.columns())
                    .map(|i| match row.get::<i64>(i)? {
                        Some(v) => Ok(PdbOwnedValue::I64(v)),
                        None => Ok(PdbOwnedValue::Null),
                    })
                    .collect::<Result<_, pgrx::spi::Error>>()?;
                counts[tree.route(&point)] += 1;
            }
            Ok::<(), pgrx::spi::Error>(())
        })
        .unwrap();
        counts
    }

    #[pg_test]
    fn boundaries_from_heap_sample_balance_the_table() {
        Spi::run(
            r#"
            CREATE TABLE sample_src (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO sample_src (tenant_id, name)
            SELECT (i * 7919) % 100, 'row ' || i FROM generate_series(1, 20000) i;
            CREATE INDEX sample_src_idx ON sample_src
                USING paradedb (id, tenant_id, name)
                WITH (partition_by = 'id, tenant_id');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = open_rels("sample_src", "sample_src_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 8)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.partition_count(), 8, "{tree}");
        assert_eq!(tree.dims().len(), 2);

        // Same heap, same seed, same boundaries.
        let BuildPartitioning::Range(again) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 8)
                .unwrap()
                .unwrap()
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(again, tree);

        // Under 4096 blocks and 30k rows, the sample is the whole table, so every partition
        // gets an equal share of the real rows.
        let counts = route_all(&tree, "sample_src", "id, tenant_id");
        for count in &counts {
            assert!(
                (2000..=3000).contains(count),
                "unbalanced partitions: {counts:?}\n{tree}"
            );
        }

        // Both declared fields end up cutting the space.
        let bounded_dims = (0..8)
            .flat_map(|p| tree.partition_bounds(p).unwrap())
            .filter(|b| !matches!(b, (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded)))
            .count();
        assert!(bounded_dims > 8, "{tree}");
    }

    #[pg_test]
    fn boundaries_follow_expression_fields_and_nulls() {
        Spi::run(
            r#"
            CREATE TABLE sample_expr (id BIGSERIAL PRIMARY KEY, base BIGINT);
            INSERT INTO sample_expr (base)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE i END FROM generate_series(1, 5000) i;
            CREATE INDEX sample_expr_idx ON sample_expr
                USING paradedb (id, ((base * 2)::pdb.alias('doubled')))
                WITH (partition_by = 'doubled');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = open_rels("sample_expr", "sample_expr_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 4)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.partition_count(), 4, "{tree}");

        // Split values come from the expression, so they are even numbers inside its range.
        let rp = tree.to_range_partitioning().unwrap();
        for sp in &rp.split_points {
            let PdbOwnedValue::I64(v) = sp else {
                panic!("expected an I64 split point, got {sp:?}");
            };
            assert!(*v % 2 == 0 && (2..=10_000).contains(v), "{tree}");
        }
        assert_eq!(tree.route(&[PdbOwnedValue::Null]), 0);
        let counts = route_all(&tree, "sample_expr", "base * 2");
        assert!(counts.iter().all(|&c| c > 0), "{counts:?}\n{tree}");
    }

    #[pg_test]
    fn no_partition_by_yields_no_tree_and_tiny_targets_stay_single() {
        Spi::run(
            r#"
            CREATE TABLE sample_plain (id BIGSERIAL PRIMARY KEY, v BIGINT);
            INSERT INTO sample_plain (v) SELECT i FROM generate_series(1, 100) i;
            CREATE INDEX sample_plain_idx ON sample_plain USING paradedb (id, v);
            CREATE TABLE sample_empty (id BIGSERIAL PRIMARY KEY, v BIGINT);
            CREATE INDEX sample_empty_idx ON sample_empty USING paradedb (id, v)
                WITH (partition_by = 'v');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = open_rels("sample_plain", "sample_plain_idx");
        assert!(
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 8)
                .unwrap()
                .is_none()
        );

        let (heaprel, indexrel) = open_rels("sample_empty", "sample_empty_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 8)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.partition_count(), 1);
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 1)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.partition_count(), 1);
    }

    /// The whole leader-to-worker path: a parallel build on a partitioned index computes and
    /// ships the boundaries without disturbing the build itself.
    #[pg_test]
    fn parallel_build_with_partition_by_succeeds() {
        Spi::run("SET max_parallel_workers = 8;").unwrap();
        Spi::run("SET max_parallel_maintenance_workers = 4;").unwrap();
        Spi::run("SET maintenance_work_mem = '256MB';").unwrap();
        Spi::run(
            r#"
            CREATE TABLE sample_parallel (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, body TEXT);
            INSERT INTO sample_parallel (tenant_id, body)
            SELECT i % 50, repeat('lorem ipsum dolor sit amet ', 20) FROM generate_series(1, 40000) i;
            CREATE INDEX sample_parallel_idx ON sample_parallel
                USING paradedb (id, tenant_id, body)
                WITH (partition_by = 'tenant_id, id', target_segment_count = 8);
            "#,
        )
        .unwrap();

        let count = Spi::get_one::<i64>(
            "SELECT count(*) FROM sample_parallel WHERE body ||| 'lorem' AND tenant_id = 3",
        )
        .unwrap()
        .unwrap();
        assert_eq!(count, 800);
    }

    /// A wide, externally TOASTed text key plus a timestamp key, with NULLs in both dimensions.
    /// Confirms the sampler detoasts under the pin, converts a timestamp, and routes NULLs low.
    #[pg_test]
    fn boundaries_detoast_text_and_timestamp_keys_with_nulls() {
        Spi::run(
            r#"
            CREATE TABLE sample_wide (id BIGSERIAL PRIMARY KEY, label TEXT, created TIMESTAMP);
            ALTER TABLE sample_wide ALTER COLUMN label SET STORAGE EXTERNAL;
            INSERT INTO sample_wide (label, created)
            SELECT
                CASE WHEN i % 11 = 0 THEN NULL
                     ELSE repeat(md5(i::text) || md5((i * 3)::text), 40) || lpad(i::text, 10, '0')
                END,
                CASE WHEN i % 7 = 0 THEN NULL
                     ELSE timestamp '2020-01-01' + make_interval(mins => i)
                END
            FROM generate_series(1, 4000) i;
            CREATE INDEX sample_wide_idx ON sample_wide
                USING paradedb (id, (label::pdb.literal), created)
                WITH (partition_by = 'label, created');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = open_rels("sample_wide", "sample_wide_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 4)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.dims().len(), 2);
        assert!((2..=4).contains(&tree.partition_count()), "{tree}");
        // NULLs in both dimensions route to the lowest partition.
        assert_eq!(tree.route(&[PdbOwnedValue::Null, PdbOwnedValue::Null]), 0);

        // Every `label` split value is a fully detoasted string of the generated width, not a
        // truncated or corrupt copy. `32 * 2` hex chars, repeated 40 times, plus a 10-char tail.
        let label_width = 32 * 2 * 40 + 10;
        let mut saw_label_split = false;
        for p in 0..tree.partition_count() {
            for (dim, (lower, upper)) in tree.partition_bounds(p).unwrap().into_iter().enumerate() {
                for bound in [lower, upper] {
                    let value = match bound {
                        std::ops::Bound::Included(v) | std::ops::Bound::Excluded(v) => Some(v),
                        std::ops::Bound::Unbounded => None,
                    };
                    if let Some(PdbOwnedValue::Str(s)) = value {
                        assert_eq!(dim, 0, "only the label dimension carries string splits");
                        assert_eq!(s.len(), label_width, "detoasted label has the wrong length");
                        saw_label_split = true;
                    }
                }
            }
        }
        assert!(
            saw_label_split,
            "expected at least one label split value: {tree}"
        );
    }

    #[pg_test]
    fn boundaries_from_heap_sample_with_ctid() {
        Spi::run(
            r#"
            CREATE TABLE sample_ctid (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO sample_ctid (tenant_id, name)
            SELECT i % 10, 'row ' || i FROM generate_series(1, 1000) i;
            CREATE INDEX sample_ctid_idx ON sample_ctid
                USING paradedb (id, tenant_id, name)
                WITH (partition_by = 'ctid');

            CREATE TABLE sample_multi_ctid (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO sample_multi_ctid (tenant_id, name)
            SELECT i % 10, 'row ' || i FROM generate_series(1, 1000) i;
            CREATE INDEX sample_multi_ctid_idx ON sample_multi_ctid
                USING paradedb (id, tenant_id, name)
                WITH (partition_by = 'tenant_id, ctid');
            "#,
        )
        .unwrap();

        let (heaprel, indexrel) = open_rels("sample_ctid", "sample_ctid_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 4)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(tree.dims(), &[FieldName::from("ctid")]);
        assert_eq!(tree.partition_count(), 4, "{tree}");

        let (heaprel, indexrel) = open_rels("sample_multi_ctid", "sample_multi_ctid_idx");
        let BuildPartitioning::Range(tree) =
            plan_partition_boundaries(&heaprel, &indexrel, snapshot_any(), 4)
                .unwrap()
                .expect("index declares partition_by")
        else {
            panic!("expected range partitioning")
        };
        assert_eq!(
            tree.dims(),
            &[FieldName::from("tenant_id"), FieldName::from("ctid")]
        );
        assert_eq!(tree.partition_count(), 4, "{tree}");

        // Both declared fields end up cutting the space.
        let bounded_dims = (0..4)
            .flat_map(|p| tree.partition_bounds(p).unwrap())
            .filter(|b| !matches!(b, (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded)))
            .count();
        assert!(bounded_dims > 4, "{tree}");
    }
}
