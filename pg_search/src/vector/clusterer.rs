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

//! SuperKMeans-backed IVF training and assignment.

use std::sync::{Arc, Mutex};

use superkmeans::{HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig};
use tantivy::vector::{
<<<<<<< HEAD
    IvfCentroids, IvfClusterer, IvfMatrix, IvfMergeSettings, IvfTrainingVectors, IvfVectors,
    Metric, VectorOptions,
=======
    IvfCentroids, IvfClusterer, IvfMatrix, IvfTrainingVectors, IvfVectors, Metric, RouterKind,
    VectorOptions,
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
};
use tantivy::{Index, TantivyError};

use crate::postgres::options::{
    BM25IndexOptions, DEFAULT_MAX_LEAF_SIZE, DEFAULT_TRAINING_SAMPLE_RATIO,
};

const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;

<<<<<<< HEAD
/// A `HierarchicalSuperKMeans` built for assignment, tagged with the
/// `(dim, angular)` it was constructed for. `assign` never reads the clusterer's
/// centroids, pruner, or cluster count — it derives everything from the vectors
/// and centroids handed to it per call — so one instance is valid for every
/// batch (and every merge) sharing the same `(dim, angular)`.
=======
/// The IVF centroid router every pg_search index builds and opens: Tantivy's
/// default relative-neighborhood graph. Tantivy persists the router kind in
/// each segment's `.centroids` file and refuses to open a segment under a
/// different kind, so this is a build-time constant, not a GUC.
pub const IVF_ROUTER: RouterKind = RouterKind::Rng;

>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
struct AssignClusterer {
    dim: usize,
    angular: bool,
    clusterer: Arc<HierarchicalSuperKMeans>,
}

#[derive(Clone)]
/// An IVF clusterer backed by hierarchical SuperKMeans.
pub struct SuperKMeansIvfClusterer {
    config: HierarchicalSuperKMeansConfig,
<<<<<<< HEAD
    centroid_ratio: f32,
    training_samples_per_centroid: usize,
    assign_batch_size: usize,
    /// Total cells a vector is written into (SPANN `ReplicaCount`). `1` (the
    /// default) is primary-only Phase 1; `> 1` adds up to `replicas - 1`
    /// next-nearest cells at merge time, selected by tantivy's centroid
    /// selector (exact scan or `RelativeNeighborhoodGraph`).
    replicas: usize,
    /// Lazily-built clusterer reused across `assign` batches.
=======
    training_sample_ratio: f32,
    assign_batch_size: usize,
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
    assign_cache: Arc<Mutex<Option<AssignClusterer>>>,
}

impl std::fmt::Debug for SuperKMeansIvfClusterer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuperKMeansIvfClusterer")
            .field("config", &self.config)
<<<<<<< HEAD
            .field("centroid_ratio", &self.centroid_ratio)
            .field(
                "training_samples_per_centroid",
                &self.training_samples_per_centroid,
            )
=======
            .field("training_sample_ratio", &self.training_sample_ratio)
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
            .field("assign_batch_size", &self.assign_batch_size)
            .field("replicas", &self.replicas)
            .finish_non_exhaustive()
    }
}

impl Default for SuperKMeansIvfClusterer {
    fn default() -> Self {
        let mut config = HierarchicalSuperKMeansConfig::default();
        config.base.suppress_warnings = true;
<<<<<<< HEAD
        config.base.sampling_fraction = 1.0;
        Self {
            config,
            centroid_ratio: 0.01,
            training_samples_per_centroid: 32,
=======
        config.max_leaf_size = DEFAULT_MAX_LEAF_SIZE as usize;
        Self {
            config,
            training_sample_ratio: DEFAULT_TRAINING_SAMPLE_RATIO as f32,
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
            assign_batch_size: DEFAULT_ASSIGN_BATCH_SIZE,
            replicas: 1,
            assign_cache: Arc::new(Mutex::new(None)),
        }
    }
}

impl SuperKMeansIvfClusterer {
    /// Creates a clusterer with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of training vectors per clustering leaf.
    pub fn with_max_leaf_size(mut self, max_leaf_size: usize) -> Self {
        self.config.max_leaf_size = max_leaf_size;
        self
    }

<<<<<<< HEAD
    pub fn with_training_samples_per_centroid(
        mut self,
        training_samples_per_centroid: usize,
    ) -> Self {
        self.training_samples_per_centroid = training_samples_per_centroid;
        self
    }

    pub fn with_replicas(mut self, replicas: usize) -> Self {
        self.replicas = replicas.max(1);
        self
    }
=======
    /// Sets the fraction of vectors sampled for training, independently of leaf size.
    pub fn with_training_sample_ratio(mut self, training_sample_ratio: f32) -> Self {
        self.training_sample_ratio = training_sample_ratio;
        self
    }
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
}

impl IvfClusterer for SuperKMeansIvfClusterer {
    fn centroid_ratio(&self) -> f32 {
        self.centroid_ratio
    }

    fn training_samples_per_centroid(&self) -> usize {
        self.training_samples_per_centroid
    }

    fn assign_batch_size(&self) -> usize {
        self.assign_batch_size
    }

<<<<<<< HEAD
    fn merge_settings(&self, total_target_docs: usize) -> tantivy::Result<IvfMergeSettings> {
        let centroid_ratio = self.centroid_ratio;
        let training_samples_per_centroid = self.training_samples_per_centroid;
        let assign_batch_size = self.assign_batch_size;

        assert!(
            centroid_ratio > 0.0 && centroid_ratio <= 1.0,
            "centroid_ratio must be in (0, 1], got {centroid_ratio}"
        );
        assert!(
            training_samples_per_centroid > 1,
            "training_samples_per_centroid must be > 1, got {training_samples_per_centroid}"
        );
        assert!(assign_batch_size > 0, "assign_batch_size must be > 0");

        let num_centroids =
            ((total_target_docs as f64) * f64::from(centroid_ratio)).ceil() as usize;
        let num_centroids = num_centroids.clamp(1, total_target_docs);

        Ok(IvfMergeSettings {
            num_centroids,
            training_samples_per_centroid,
            assign_batch_size,
            // Replica cells (the `replicas - 1` non-primary cells per vector)
            // are selected by tantivy in the field's raw metric —
            // router-consistent with query-time `rank_centroids`. No angular
            // assumption on this clusterer remains.
            replicas: self.replicas.max(1),
        })
    }

=======
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
    fn train(
        &self,
        options: &VectorOptions,
        vectors: IvfTrainingVectors,
        num_centroids: usize,
    ) -> tantivy::Result<IvfCentroids> {
        let IvfTrainingVectors::F32(vectors) = vectors;
        let dim = options.dim();
        if vectors.matrix.dims != dim {
            return Err(TantivyError::InvalidArgument(format!(
                "vector dimensionality mismatch: expected {dim}, got {}",
                vectors.matrix.dims
            )));
        }
        if vectors.doc_ids.len() != vectors.matrix.rows {
            return Err(TantivyError::InvalidArgument(format!(
                "vector doc_id count mismatch: expected {}, got {}",
                vectors.matrix.rows,
                vectors.doc_ids.len()
            )));
        }
        if vectors.matrix.values.len() != vectors.matrix.rows * dim {
            return Err(TantivyError::InvalidArgument(format!(
                "vector value count mismatch: expected {}, got {}",
                vectors.matrix.rows * dim,
                vectors.matrix.values.len()
            )));
        }

        let mut config = self.config.clone();
        if matches!(options.metric(), Metric::Cosine | Metric::Dot) {
            config.base.angular = true;
        }
        let mut clusterer = HierarchicalSuperKMeans::with_config(num_centroids, dim, config);
        let rows = vectors.matrix.rows;
<<<<<<< HEAD
        // Hand the buffer to superkmeans so it can rotate in place instead of
        // keeping a second full-size copy alive through training.
        let centroids = clusterer.train_owned(vectors.matrix.values, rows);
        if centroids.len() != num_centroids * dim {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned {} centroid floats, expected {}",
                centroids.len(),
                num_centroids * dim
            )));
        }
=======
        let centroids = clusterer.train_owned(vectors.matrix.values, rows);
        if centroids.is_empty() || !centroids.len().is_multiple_of(dim) {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned an invalid centroid matrix with {} floats for dimension {}",
                centroids.len(),
                dim
            )));
        }
        let num_centroids = centroids.len() / dim;
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
        Ok(IvfCentroids::F32(IvfMatrix {
            values: centroids,
            rows: num_centroids,
            dims: dim,
        }))
    }

    fn assign(
        &self,
        options: &VectorOptions,
        vectors: IvfVectors<'_>,
        centroids: &IvfCentroids,
    ) -> tantivy::Result<Vec<u32>> {
        let IvfVectors::F32(vectors) = vectors;
        let IvfCentroids::F32(centroids) = centroids;
        let dim = options.dim();
        let vector_matrix = vectors.matrix;
        let centroid_matrix = centroids;
        if vector_matrix.dims != dim {
            return Err(TantivyError::InvalidArgument(format!(
                "vector dimensionality mismatch: expected {dim}, got {}",
                vector_matrix.dims
            )));
        }
        if vectors.doc_ids.len() != vector_matrix.rows {
            return Err(TantivyError::InvalidArgument(format!(
                "vector doc_id count mismatch: expected {}, got {}",
                vector_matrix.rows,
                vectors.doc_ids.len()
            )));
        }
        if vector_matrix.values.len() != vector_matrix.rows * dim {
            return Err(TantivyError::InvalidArgument(format!(
                "vector value count mismatch: expected {}, got {}",
                vector_matrix.rows * dim,
                vector_matrix.values.len()
            )));
        }
        if centroid_matrix.rows == 0 {
            return Err(TantivyError::InvalidArgument(
                "cannot assign with zero centroids".to_string(),
            ));
        }
        if centroid_matrix.dims != dim {
            return Err(TantivyError::InvalidArgument(format!(
                "centroid dimensionality mismatch: expected {dim}, got {}",
                centroid_matrix.dims
            )));
        }
        if centroid_matrix.values.len() != centroid_matrix.rows * dim {
            return Err(TantivyError::InvalidArgument(format!(
                "centroid value count mismatch: expected {}, got {}",
                centroid_matrix.rows * dim,
                centroid_matrix.values.len()
            )));
        }
        if vector_matrix.rows == 0 {
            return Ok(Vec::new());
        }

        let angular = matches!(options.metric(), Metric::Cosine | Metric::Dot);

        let clusterer = {
            let mut cache = self
                .assign_cache
                .lock()
                .expect("assign clusterer cache mutex poisoned");
            match cache.as_ref() {
                Some(entry) if entry.dim == dim && entry.angular == angular => {
                    entry.clusterer.clone()
                }
                _ => {
                    let mut config = self.config.clone();
                    config.base.angular = angular;
                    let clusterer = Arc::new(HierarchicalSuperKMeans::with_config(
                        centroid_matrix.rows,
                        dim,
                        config,
                    ));
                    *cache = Some(AssignClusterer {
                        dim,
                        angular,
                        clusterer: clusterer.clone(),
                    });
                    clusterer
                }
            }
        };
<<<<<<< HEAD
        // Primary (nearest-centroid) assignment via superkmeans, angular-aware
        // for cosine/dot. One cluster per vector — no replication. `n_centroids`
        // is derived from the centroid slice length.
=======
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
        let primaries = clusterer.assign(
            vector_matrix.values,
            centroid_matrix.values.as_slice(),
            vector_matrix.rows,
        );
        Ok(primaries)
    }
}

<<<<<<< HEAD
pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_centroid_ratio(options.centroid_ratio())
        .with_training_samples_per_centroid(options.training_samples_per_centroid())
        .with_replicas(options.cluster_replication());
=======
/// Select the IVF router on an opened index. Every `Index::open` in pg_search
/// goes through this: tantivy requires a configured router both to build IVF
/// segments at merge time and to open existing ones for search.
pub fn set_ivf_router(index: &mut Index) -> tantivy::Result<()> {
    index.set_ivf_router(IVF_ROUTER)
}

/// Installs the configured IVF clusterer on an index.
pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_max_leaf_size(options.max_leaf_size())
        .with_training_sample_ratio(options.training_sample_ratio());
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))
    index.set_ivf_clusterer(Arc::new(clusterer));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Replication is off by default (`replicas = 1`), and non-positive
    /// configured values clamp to `1` rather than disabling clustering.
    #[test]
<<<<<<< HEAD
    fn replicas_default_and_clamp() {
        let total = 100_000;
        let settings = SuperKMeansIvfClusterer::new()
            .merge_settings(total)
            .unwrap();
        assert_eq!(settings.replicas, 1, "primary-only by default");
=======
    fn default_training_settings() {
        let clusterer = SuperKMeansIvfClusterer::default();
        let settings = clusterer.merge_settings(10_000).unwrap();
        assert_eq!(settings.training_sample_ratio, 0.32);
        assert_eq!(settings.assign_batch_size, DEFAULT_ASSIGN_BATCH_SIZE);
        assert_eq!(clusterer.config.max_leaf_size, 100);
    }

    #[test]
    fn leaf_size_and_training_fraction_are_independent() {
        let clusterer = SuperKMeansIvfClusterer::new()
            .with_max_leaf_size(20)
            .with_training_sample_ratio(0.25);
        for total_docs in [100, 10_000] {
            let settings = clusterer.merge_settings(total_docs).unwrap();
            assert_eq!(settings.training_sample_ratio, 0.25);
        }
        assert_eq!(clusterer.config.max_leaf_size, 20);
        let larger_leaves = clusterer.with_max_leaf_size(200);
        assert_eq!(larger_leaves.training_sample_ratio(), 0.25);
        assert_eq!(larger_leaves.config.max_leaf_size, 200);
        let larger_sample = larger_leaves.with_training_sample_ratio(0.5);
        assert_eq!(larger_sample.training_sample_ratio(), 0.5);
        assert_eq!(larger_sample.config.max_leaf_size, 200);
    }

    #[test]
    fn full_sample_ratio_reaches_tantivy_unchanged() {
        let clusterer = SuperKMeansIvfClusterer::new().with_training_sample_ratio(1.0);
        assert_eq!(
            clusterer
                .merge_settings(10_000)
                .unwrap()
                .training_sample_ratio,
            1.0
        );
        assert_eq!(clusterer.config.max_leaf_size, 100);
    }
>>>>>>> a5497ad9f (feat(vector): quantized vector search, on by default (#6177))

        let replicated = SuperKMeansIvfClusterer::new()
            .with_replicas(4)
            .merge_settings(total)
            .unwrap();
        assert_eq!(replicated.replicas, 4);

        let clamped = SuperKMeansIvfClusterer::new()
            .with_replicas(0)
            .merge_settings(total)
            .unwrap();
        assert_eq!(clamped.replicas, 1, "non-positive clamps to primary-only");
    }
}
