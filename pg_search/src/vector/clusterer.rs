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

//! CREATE INDEX-time centroid training.
//!
//! Centroids are an index-level artifact under tantivy's V3 vector format:
//! trained ONCE, over a sample of the whole corpus, installed at index
//! creation like the schema and settings, and never retrained — segments
//! only assign against them (at commit and merge, inside tantivy). This
//! module owns the training side: a reservoir sampler fed by the
//! CREATE INDEX heap scan, the hierarchical-superkmeans run per vector
//! field, and the [`CentroidIndex`] provider handed to
//! `IndexBuilder::centroid_index`.

use crate::api::HashMap;
use crate::postgres::options::BM25IndexOptions;
use crate::vector::PgVector;
use anyhow::{Result, bail};
use pgrx::{FromDatum, pg_sys};
use superkmeans::{HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig};
use tantivy::TantivyError;
use tantivy::schema::Field;
use tantivy::vector::{CentroidIndex, IvfCentroids, IvfMatrix, Metric, VectorOptions};

/// Floor on reservoir capacity, so a tiny table still trains on whatever
/// it has rather than on a couple of rows.
const MIN_RESERVOIR_ROWS: usize = 1024;

/// The version stamped on the (only) centroid set an index ever gets.
/// Re-publishing — background reclustering under a bumped version — does
/// not exist yet; a REINDEX retrains from scratch, still at this version.
pub const INITIAL_CENTROID_SET_VERSION: u64 = 1;

/// The frozen training result for every vector field of the schema,
/// pulled by tantivy exactly once at index creation.
pub struct TrainedCentroidIndex {
    fields: HashMap<Field, TrainedField>,
}

struct TrainedField {
    values: Vec<f32>,
    rows: usize,
    dim: usize,
}

impl CentroidIndex for TrainedCentroidIndex {
    fn version(&self) -> u64 {
        INITIAL_CENTROID_SET_VERSION
    }

    fn centroids(&self, field: Field, options: &VectorOptions) -> tantivy::Result<IvfCentroids> {
        let Some(trained) = self.fields.get(&field) else {
            return Err(TantivyError::InternalError(format!(
                "no centroids were trained for vector field {field:?}"
            )));
        };
        debug_assert_eq!(trained.dim, options.dim());
        Ok(IvfCentroids::F32(IvfMatrix {
            values: trained.values.clone(),
            rows: trained.rows,
            dims: trained.dim,
        }))
    }
}

/// One vector field of the index being built, as `create_index`'s schema
/// loop discovered it: the tantivy [`Field`] it was assigned, and its
/// position within the index's column order (the build callback's
/// `values` array).
pub struct SampledFieldSpec {
    pub ordinal: usize,
    pub field: Field,
    pub field_name: String,
    pub dim: usize,
    pub metric: Metric,
}

/// One vector field's reservoir during the sampling heap scan.
struct SampledField {
    spec: SampledFieldSpec,
    /// Reservoir capacity, in rows.
    cap: usize,
    /// Vector-bearing rows seen (not sampled) — the training-floor count
    /// and the reservoir's algorithm-R denominator.
    seen: usize,
    /// The reservoir: `min(seen, cap)` rows, `dim`-strided.
    rows: Vec<f32>,
}

/// Uniform reservoir sampler over the CREATE INDEX heap scan, one
/// reservoir per vector field. Capacity is bounded by
/// `maintenance_work_mem` (half of it, split across vector fields) — the
/// same knob pgvector builds are sized with.
pub struct VectorSampler {
    fields: Vec<SampledField>,
    /// `centroid_ratio * training_samples_per_centroid`: the fraction of
    /// the corpus the reservoirs retain. Capacity is derived from the
    /// rows SEEN SO FAR rather than a row estimate, so it needs no
    /// ANALYZE and lands at `num_centroids * training_samples_per_centroid`
    /// exactly when the scan ends.
    sample_fraction: f64,
    /// xorshift64* state; fixed seed, so a rebuild of the same data
    /// samples the same rows.
    rng: u64,
}

impl VectorSampler {
    /// Reservoirs for the given vector fields (from `create_index`'s
    /// schema loop). `specs` must be non-empty. Sizing honors
    /// `training_samples_per_centroid`: each reservoir retains
    /// `centroid_ratio * training_samples_per_centroid` of the corpus, so
    /// training ends up with that many samples per centroid.
    pub fn from_specs(specs: Vec<SampledFieldSpec>, options: &BM25IndexOptions) -> Self {
        assert!(!specs.is_empty(), "sampling requires vector fields");
        let sample_fraction =
            f64::from(options.centroid_ratio()) * options.training_samples_per_centroid() as f64;
        let fields = specs
            .into_iter()
            .map(|spec| SampledField {
                spec,
                cap: MIN_RESERVOIR_ROWS,
                seen: 0,
                rows: Vec::new(),
            })
            .collect();
        VectorSampler {
            fields,
            sample_fraction,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_rng(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Offer one heap row (the build callback's `values`/`isnull` arrays,
    /// in index-column order) to every field's reservoir.
    ///
    /// # Safety
    ///
    /// `values` and `isnull` must be the arrays Postgres passes to an
    /// `IndexBuildCallback`, valid for the index's column count.
    pub unsafe fn offer(&mut self, values: *mut pg_sys::Datum, isnull: *mut bool) {
        for i in 0..self.fields.len() {
            let ordinal = self.fields[i].spec.ordinal;
            if *isnull.add(ordinal) {
                continue;
            }
            let datum = *values.add(ordinal);
            let Some(vector) = PgVector::from_datum(datum, false) else {
                continue;
            };
            let field = &self.fields[i];
            if vector.0.len() != field.spec.dim {
                panic!(
                    "vector field '{}' expects {} dimensions, got {}",
                    field.spec.field_name,
                    field.spec.dim,
                    vector.0.len()
                );
            }
            // Capacity tracks the rows seen so far, so it converges on
            // `num_centroids * training_samples_per_centroid` without
            // needing to know the row count up front. It only ever grows;
            // the freed slots are filled by subsequent rows.
            let dim = field.spec.dim;
            let seen = field.seen;
            let cap = (((seen + 1) as f64 * self.sample_fraction).ceil() as usize)
                .max(MIN_RESERVOIR_ROWS);
            // Occupied slots, which must stay DENSE: appending is the only
            // way the reservoir grows, so a widened capacity is filled by
            // subsequent rows instead of leaving zero-filled holes for
            // k-means to train on.
            let filled = field.rows.len() / dim;
            let slot = if filled < cap {
                Some(filled)
            } else {
                // Algorithm R: replace a random resident with probability
                // cap / (seen + 1).
                let j = (self.next_rng() % (seen as u64 + 1)) as usize;
                (j < cap).then_some(j)
            };
            let field = &mut self.fields[i];
            field.cap = cap;
            if let Some(slot) = slot {
                let start = slot * dim;
                debug_assert!(start <= field.rows.len(), "reservoir must stay dense");
                if field.rows.len() == start {
                    field.rows.extend_from_slice(&vector.0);
                } else {
                    field.rows[start..start + dim].copy_from_slice(&vector.0);
                }
            }
            field.seen += 1;
        }
    }

    /// Vector-bearing rows seen per field, for the training floor.
    pub fn rows_seen(&self) -> impl Iterator<Item = (&str, usize)> {
        self.fields
            .iter()
            .map(|field| (field.spec.field_name.as_str(), field.seen))
    }

    /// Train each field's centroids over its reservoir. `centroid_ratio`
    /// and the target centroid count are resolved against the TRUE row
    /// count (`seen`), not the reservoir size.
    pub fn train(self, options: &BM25IndexOptions) -> Result<TrainedCentroidIndex> {
        let centroid_ratio = options.centroid_ratio();
        let mut fields = HashMap::default();
        for sampled in self.fields {
            let spec = sampled.spec;
            // The reservoir is dense, so its length IS the sample size.
            let sampled_rows = sampled.rows.len() / spec.dim;
            if sampled_rows == 0 {
                bail!(
                    "vector field '{}' has no vectors to train on",
                    spec.field_name
                );
            }
            let num_centroids = ((sampled.seen as f64) * f64::from(centroid_ratio))
                .ceil()
                .max(1.0) as usize;
            let num_centroids = num_centroids.clamp(1, sampled_rows);

            let mut values = sampled.rows;
            debug_assert_eq!(values.len(), sampled_rows * spec.dim);
            let angular = matches!(spec.metric, Metric::Cosine | Metric::Dot);
            if spec.metric == Metric::Cosine {
                // Mirror the stored-row contract: rows are unit-normalized
                // at ingest, so train in the same space.
                for row in values.chunks_exact_mut(spec.dim) {
                    let norm = row
                        .iter()
                        .map(|x| f64::from(*x) * f64::from(*x))
                        .sum::<f64>()
                        .sqrt();
                    if norm.is_finite() && norm > 0.0 {
                        for x in row {
                            *x = (f64::from(*x) / norm) as f32;
                        }
                    }
                }
            }

            let mut config = HierarchicalSuperKMeansConfig::default();
            config.base.suppress_warnings = true;
            config.base.sampling_fraction = 1.0;
            config.base.angular = angular;
            let mut clusterer =
                HierarchicalSuperKMeans::with_config(num_centroids, spec.dim, config);
            let centroids = clusterer.train(&values, sampled_rows);
            if centroids.len() != num_centroids * spec.dim {
                bail!(
                    "SuperKMeans returned {} centroid floats for field '{}', expected {}",
                    centroids.len(),
                    spec.field_name,
                    num_centroids * spec.dim
                );
            }
            pgrx::debug1!(
                "trained {num_centroids} centroids for vector field '{}' over {sampled_rows} \
                 sampled rows ({} seen)",
                spec.field_name,
                sampled.seen,
            );
            fields.insert(
                spec.field,
                TrainedField {
                    values: centroids,
                    rows: num_centroids,
                    dim: spec.dim,
                },
            );
        }
        Ok(TrainedCentroidIndex { fields })
    }
}
