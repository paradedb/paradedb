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

#![allow(dead_code)]
//! Hash-routing primitives for MPP shuffle.
//!
//! Two production implementations:
//! - [`HashPartitioner`] — hashes one or more key columns, assigns each row
//!   to `hash % total_participants`. Uses
//!   `ahash::RandomState::with_seeds(0, 0, 0, 0)` so routing matches
//!   DataFusion's `REPARTITION_RANDOM_STATE`.
//! - [`FixedTargetPartitioner`] — routes every row to a single target,
//!   used for the scalar-final gather (workers ship Partial rows to the
//!   leader's `FinalPartitioned` aggregate).
//!
//! [`split_batch_by_partition`] takes the per-row destination vector that
//! a partitioner returns and produces one sub-batch per destination, ready
//! to be encoded and shipped down the mesh.
//!
//! Extracted from the legacy `shuffle.rs` (which `MppShuffleExec` used to
//! live in); the producer-side ExecutionPlan now lives in
//! [`super::shm_mq_producer`].

use datafusion::arrow::array::{RecordBatch, UInt64Array};
use datafusion::arrow::compute::take;
use datafusion::common::hash_utils::create_hashes;
use datafusion::common::DataFusionError;

/// Assigns each row of a `RecordBatch` to one of N destination participants.
///
/// Implementations must return exactly `batch.num_rows()` values, each in
/// `0..total_participants`.
///
/// TODO(future): align this with DataFusion's own `Partitioning` so the
/// routing decision rides on the optimizer's existing partition concept
/// rather than a parallel one. The destination model
/// (`datafusion-distributed`'s `PartitionIsolatorExec`) is to express
/// participant identity as a `Partitioning::Hash` partition that the
/// transport peels off, which would let us delete this trait.
pub trait RowPartitioner: Send + Sync {
    /// Return a destination index in `0..total_participants` for every row.
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError>;

    /// Total destinations this partitioner will target. Used by callers to
    /// size per-destination scratch arrays.
    fn total_participants(&self) -> u32;
}

/// Production partitioner: hashes one or more key columns and assigns each
/// row to `hash % total_participants`.
///
/// Uses `create_hashes` with `ahash::RandomState::with_seeds(0, 0, 0, 0)` —
/// the same seed DataFusion uses for `REPARTITION_RANDOM_STATE`, so routing
/// is stable across workers.
///
/// NULL keys hash to a single sentinel, so a heavily-NULL key column skews
/// destinations. Correct (the receiving HashJoin/Aggregate clusters NULL keys
/// the same way), but worth checking trace metrics if a hot peer shows up.
///
/// TODO: accept `Vec<Arc<dyn PhysicalExpr>>` instead of column indices so the
/// routing stays byte-compatible if a planner pushes `CAST(col)` or similar
/// into the key list. Today we only accept column refs.
pub struct HashPartitioner {
    /// Indices into the input schema for the key columns to hash.
    key_columns: Vec<usize>,
    total_participants: u32,
}

impl HashPartitioner {
    pub fn new(key_columns: Vec<usize>, total_participants: u32) -> Self {
        assert!(
            total_participants > 0,
            "HashPartitioner requires total_participants >= 1"
        );
        Self {
            key_columns,
            total_participants,
        }
    }
}

impl RowPartitioner for HashPartitioner {
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
        if self.total_participants == 1 {
            // Single participant degenerate case — everyone is self.
            return Ok(vec![0u32; batch.num_rows()]);
        }
        if self.key_columns.is_empty() {
            return Err(DataFusionError::Internal(
                "HashPartitioner: no key columns provided".into(),
            ));
        }
        let columns: Vec<_> = self
            .key_columns
            .iter()
            .map(|&idx| batch.column(idx).clone())
            .collect();
        let mut hashes_buf = vec![0u64; batch.num_rows()];
        let random_state = ahash::RandomState::with_seeds(0, 0, 0, 0);
        create_hashes(&columns, &random_state, &mut hashes_buf)?;
        let n = self.total_participants as u64;
        Ok(hashes_buf.into_iter().map(|h| (h % n) as u32).collect())
    }

    fn total_participants(&self) -> u32 {
        self.total_participants
    }
}

/// Route every row to one fixed destination. Used by the scalar-aggregate
/// final-gather: workers ship `Partial` rows to the leader's participant for
/// `FinalPartitioned`.
pub struct FixedTargetPartitioner {
    target: u32,
    total_participants: u32,
}

impl FixedTargetPartitioner {
    pub fn new(target: u32, total_participants: u32) -> Self {
        assert!(total_participants > 0);
        assert!(
            target < total_participants,
            "FixedTargetPartitioner: target {target} >= total_participants {total_participants}"
        );
        Self {
            target,
            total_participants,
        }
    }
}

impl RowPartitioner for FixedTargetPartitioner {
    fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
        Ok(vec![self.target; batch.num_rows()])
    }

    fn total_participants(&self) -> u32 {
        self.total_participants
    }
}

/// Scatter a `RecordBatch` into one sub-batch per destination participant.
///
/// Returns a `Vec` of length `total_participants`. Each entry is either:
/// - `Some(sub_batch)` if one or more rows of `batch` routed to that
///   participant; or
/// - `None` if no rows routed to that participant (skip a send round-trip).
///
/// Within each sub-batch, the original row order is preserved.
pub fn split_batch_by_partition(
    batch: &RecordBatch,
    destinations: &[u32],
    total_participants: u32,
) -> Result<Vec<Option<RecordBatch>>, DataFusionError> {
    if destinations.len() != batch.num_rows() {
        return Err(DataFusionError::Internal(format!(
            "split_batch_by_partition: destinations.len()={} != batch.num_rows()={}",
            destinations.len(),
            batch.num_rows()
        )));
    }
    if total_participants == 0 {
        return Err(DataFusionError::Internal(
            "split_batch_by_partition: total_participants must be > 0".into(),
        ));
    }

    let n = total_participants as usize;
    let mut buckets: Vec<Option<Vec<u64>>> = (0..n).map(|_| None).collect();
    for (row_idx, &dest) in destinations.iter().enumerate() {
        if dest as usize >= n {
            return Err(DataFusionError::Internal(format!(
                "split_batch_by_partition: destination {dest} >= total_participants {total_participants}"
            )));
        }
        buckets[dest as usize]
            .get_or_insert_with(Vec::new)
            .push(row_idx as u64);
    }

    let schema = batch.schema();
    let mut out: Vec<Option<RecordBatch>> = Vec::with_capacity(n);
    for bucket in buckets {
        match bucket {
            None => out.push(None),
            Some(indices) => {
                let idx_array = UInt64Array::from(indices);
                let taken_cols: Result<Vec<_>, DataFusionError> = batch
                    .columns()
                    .iter()
                    .map(|c| {
                        take(c.as_ref(), &idx_array, None)
                            .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))
                    })
                    .collect();
                let taken_cols = taken_cols?;
                let sub = RecordBatch::try_new(schema.clone(), taken_cols)
                    .map_err(|e| DataFusionError::ArrowError(Box::new(e), None))?;
                out.push(Some(sub));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use datafusion::arrow::array::{Int32Array, StringArray};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    /// Test-only partitioner: row `i` -> destination `i % total_participants`.
    ///
    /// Not suitable for production because adjacent rows land on different
    /// participants regardless of their join key, which would break
    /// HashJoin/Aggregate correctness. Useful in unit tests because the
    /// routing is trivially predictable without committing to a specific
    /// hash output.
    pub struct ModuloPartitioner {
        total_participants: u32,
    }

    impl ModuloPartitioner {
        pub fn new(total_participants: u32) -> Self {
            assert!(total_participants > 0);
            Self { total_participants }
        }
    }

    impl RowPartitioner for ModuloPartitioner {
        fn partition_for_each_row(&self, batch: &RecordBatch) -> Result<Vec<u32>, DataFusionError> {
            let n = self.total_participants;
            Ok((0..batch.num_rows() as u32).map(|i| i % n).collect())
        }

        fn total_participants(&self) -> u32 {
            self.total_participants
        }
    }

    fn sample_batch(rows: i32) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        let ids = Int32Array::from_iter_values(0..rows);
        let names = StringArray::from_iter_values((0..rows).map(|i| format!("n{i}")));
        RecordBatch::try_new(schema, vec![Arc::new(ids), Arc::new(names)]).unwrap()
    }

    #[test]
    fn modulo_partitioner_round_robins() {
        let batch = sample_batch(7);
        let p = ModuloPartitioner::new(3);
        let dests = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests, vec![0, 1, 2, 0, 1, 2, 0]);
    }

    #[test]
    fn split_batch_by_partition_preserves_order() {
        let batch = sample_batch(6);
        let dests = vec![0, 1, 0, 1, 0, 1];
        let out = split_batch_by_partition(&batch, &dests, 2).unwrap();
        assert_eq!(out.len(), 2);
        let p0 = out[0].as_ref().unwrap();
        let p1 = out[1].as_ref().unwrap();
        assert_eq!(p0.num_rows(), 3);
        assert_eq!(p1.num_rows(), 3);
        let ids_p0 = p0.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(ids_p0.values(), &[0, 2, 4]);
        let ids_p1 = p1.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
        assert_eq!(ids_p1.values(), &[1, 3, 5]);
    }

    #[test]
    fn split_batch_by_partition_returns_none_for_empty_destinations() {
        let batch = sample_batch(4);
        let dests = vec![0, 0, 0, 0];
        let out = split_batch_by_partition(&batch, &dests, 3).unwrap();
        assert!(out[0].is_some());
        assert!(out[1].is_none());
        assert!(out[2].is_none());
    }

    #[test]
    fn split_batch_by_partition_rejects_length_mismatch() {
        let batch = sample_batch(4);
        let dests = vec![0, 1];
        assert!(split_batch_by_partition(&batch, &dests, 2).is_err());
    }

    #[test]
    fn split_batch_by_partition_rejects_out_of_range_destination() {
        let batch = sample_batch(3);
        let dests = vec![0, 5, 0];
        assert!(split_batch_by_partition(&batch, &dests, 2).is_err());
    }

    #[test]
    fn hash_partitioner_is_deterministic() {
        let batch = sample_batch(100);
        let p = HashPartitioner::new(vec![0], 4);
        let dests_a = p.partition_for_each_row(&batch).unwrap();
        let dests_b = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests_a, dests_b);
        assert_eq!(dests_a.len(), 100);
        for d in &dests_a {
            assert!(*d < 4);
        }
    }

    #[test]
    fn hash_partitioner_is_deterministic_with_multi_column_keys() {
        let batch = sample_batch(64);
        let p = HashPartitioner::new(vec![0, 1], 4);
        let dests_a = p.partition_for_each_row(&batch).unwrap();
        let dests_b = p.partition_for_each_row(&batch).unwrap();
        assert_eq!(dests_a, dests_b);
        assert_eq!(dests_a.len(), 64);
        for d in &dests_a {
            assert!(*d < 4);
        }
        let mut hits = [false; 4];
        for d in &dests_a {
            hits[*d as usize] = true;
        }
        assert!(hits.iter().all(|h| *h));
    }

    #[test]
    fn hash_partitioner_degenerate_n1_routes_all_to_zero() {
        let batch = sample_batch(50);
        let p = HashPartitioner::new(vec![0], 1);
        let dests = p.partition_for_each_row(&batch).unwrap();
        assert!(dests.iter().all(|&d| d == 0));
    }

    #[test]
    fn hash_partitioner_requires_key_columns() {
        let batch = sample_batch(10);
        let p = HashPartitioner::new(vec![], 2);
        assert!(p.partition_for_each_row(&batch).is_err());
    }

    #[test]
    fn hash_partitioner_distributes_across_participants() {
        let batch = sample_batch(1000);
        let p = HashPartitioner::new(vec![0], 4);
        let dests = p.partition_for_each_row(&batch).unwrap();
        let mut counts = [0u32; 4];
        for d in dests {
            counts[d as usize] += 1;
        }
        for c in counts {
            assert!(c > 100, "bucket too small: counts={counts:?}");
        }
    }

    #[test]
    fn fixed_target_partitioner_routes_all_to_target() {
        let batch = sample_batch(20);
        let p = FixedTargetPartitioner::new(2, 4);
        let dests = p.partition_for_each_row(&batch).unwrap();
        assert!(dests.iter().all(|&d| d == 2));
    }
}
