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

pub mod metric;

use metric::VectorMetric;
use pgrx::{FromDatum, pg_sys};
use superkmeans::distance::l2_squared;

/// Owned `Vec<f32>` extracted from a pgvector datum. pgvector's on-disk
/// layout (after the varlena header) is `[int16 dim][int16 unused]
/// [float4; dim]`. `from_datum` detoasts the datum, copies the floats
/// out, and frees the detoasted copy if it differs from the original
/// pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct PgVector(pub Vec<f32>);

impl PgVector {
    /// Leaves non-cosine vectors unchanged. Returns false without modifying the vector
    /// when cosine normalization is undefined (zero or non-finite norm).
    #[must_use]
    pub fn normalize_for_metric(&mut self, metric: metric::VectorMetric) -> bool {
        if metric != metric::VectorMetric::Cosine {
            return true;
        }
        let norm = self
            .0
            .iter()
            .map(|&v| f64::from(v).powi(2))
            .sum::<f64>()
            .sqrt();
        if norm == 0.0 || !norm.is_finite() {
            return false;
        }
        for value in &mut self.0 {
            *value = (f64::from(*value) / norm) as f32;
        }
        true
    }

    /// Angular partition centroids are normalized, so the query norm does not affect their order.
    pub(crate) fn partition_distance(&self, metric: VectorMetric, centroid: &[f32]) -> f32 {
        match metric {
            VectorMetric::L2 => l2_squared(&self.0, centroid),
            VectorMetric::Cosine | VectorMetric::InnerProduct => -self.dot(centroid),
        }
    }

    pub fn dot(&self, other: &[f32]) -> f32 {
        assert_eq!(self.0.len(), other.len(), "vector dimension mismatch");
        let mut sums = [0.0; 8];
        let (rows, row_tail) = self.0.as_chunks::<8>();
        let (others, other_tail) = other.as_chunks::<8>();
        for (row, other) in rows.iter().zip(others) {
            for lane in 0..8 {
                sums[lane] += row[lane] * other[lane];
            }
        }
        let tail: f32 = row_tail.iter().zip(other_tail).map(|(a, b)| a * b).sum();
        sums.iter().sum::<f32>() + tail
    }
}

impl FromDatum for PgVector {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }
        let ptr = datum.cast_mut_ptr::<pg_sys::varlena>();
        let detoasted = pg_sys::pg_detoast_datum(ptr);
        let data = pgrx::varlena::vardata_any(detoasted);
        let dim = *(data as *const i16) as usize;
        let floats_ptr = data.add(4) as *const f32;
        let floats = std::slice::from_raw_parts(floats_ptr, dim).to_vec();
        if detoasted != ptr {
            pg_sys::pfree(detoasted as *mut std::ffi::c_void);
        }
        Some(Self(floats))
    }
}

#[cfg(test)]
mod tests {
    use super::{PgVector, metric::VectorMetric};

    #[test]
    fn cosine_normalization_handles_extreme_finite_values() {
        for scale in [1.0, f32::MAX / 4.0, f32::MIN_POSITIVE, f32::from_bits(1)] {
            let mut vector = PgVector(vec![3.0 * scale, -4.0 * scale]);
            assert!(vector.normalize_for_metric(VectorMetric::Cosine));
            assert!((vector.0[0] - 0.6).abs() < 1e-6);
            assert!((vector.0[1] + 0.8).abs() < 1e-6);
        }
    }

    #[test]
    fn undefined_cosine_normalization_preserves_input() {
        for values in [
            vec![],
            vec![0.0, -0.0],
            vec![1.0, f32::INFINITY],
            vec![f32::NAN],
        ] {
            let original: Vec<_> = values.iter().map(|v| v.to_bits()).collect();
            let mut vector = PgVector(values);
            assert!(!vector.normalize_for_metric(VectorMetric::Cosine));
            assert_eq!(
                vector.0.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                original
            );
        }
    }

    #[test]
    fn non_cosine_metrics_preserve_magnitude_and_zero_vectors() {
        for metric in [VectorMetric::L2, VectorMetric::InnerProduct] {
            for values in [vec![3.0, 4.0], vec![0.0, 0.0]] {
                let mut vector = PgVector(values.clone());
                assert!(vector.normalize_for_metric(metric));
                assert_eq!(vector.0, values);
            }
        }
    }

    #[test]
    fn dot_covers_chunk_boundaries() {
        for len in [0, 1, 7, 8, 9, 16, 19] {
            let vector = PgVector((0..len).map(|i| i as f32 - 5.0).collect());
            let other: Vec<_> = (0..len).map(|i| 2.0 * i as f32 + 1.0).collect();
            let expected: f32 = vector.0.iter().zip(&other).map(|(a, b)| a * b).sum();
            assert_eq!(vector.dot(&other), expected, "length {len}");
        }
    }

    #[test]
    #[should_panic(expected = "vector dimension mismatch")]
    fn dot_rejects_mismatched_dimensions() {
        PgVector(vec![1.0, 2.0]).dot(&[3.0]);
    }
}
