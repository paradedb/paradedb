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

use std::sync::{Arc, Mutex};

use superkmeans::{HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig};
use tantivy::vector::{
    IvfCentroids, IvfClusterer, IvfMatrix, IvfMergeSettings, IvfTrainingVectors, IvfVectors,
    Metric, RouterKind, VectorOptions,
};
use tantivy::{Index, TantivyError};

use crate::postgres::options::BM25IndexOptions;

const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;

/// The IVF centroid router every pg_search index builds and opens: a stacked
/// IVF over the trained centroids. Tantivy persists the router kind in each
/// segment's `.centroids` file and refuses to open a segment under a
/// different kind, so this is a build-time constant, not a GUC.
pub const IVF_ROUTER: RouterKind = RouterKind::Stacked;

/// A `HierarchicalSuperKMeans` built for assignment, tagged with the
/// `(dim, angular)` it was constructed for. `assign` never reads the clusterer's
/// centroids, pruner, or cluster count — it derives everything from the vectors
/// and centroids handed to it per call — so one instance is valid for every
/// batch (and every merge) sharing the same `(dim, angular)`.
struct AssignClusterer {
    dim: usize,
    angular: bool,
    clusterer: Arc<HierarchicalSuperKMeans>,
}

#[derive(Clone)]
pub struct SuperKMeansIvfClusterer {
    config: HierarchicalSuperKMeansConfig,
    centroid_ratio: f32,
    training_sample_ratio: f32,
    assign_batch_size: usize,
    /// Lazily-built clusterer reused across `assign` batches.
    assign_cache: Arc<Mutex<Option<AssignClusterer>>>,
}

impl std::fmt::Debug for SuperKMeansIvfClusterer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuperKMeansIvfClusterer")
            .field("config", &self.config)
            .field("centroid_ratio", &self.centroid_ratio)
            .field("training_sample_ratio", &self.training_sample_ratio)
            .field("assign_batch_size", &self.assign_batch_size)
            .finish_non_exhaustive()
    }
}

impl Default for SuperKMeansIvfClusterer {
    fn default() -> Self {
        // Per-run knobs live on the nested `base` config in superkmeans-rs.
        let mut config = HierarchicalSuperKMeansConfig::default();
        config.base.suppress_warnings = true;
        Self {
            config,
            centroid_ratio: 0.01,
            training_sample_ratio: 0.32,
            assign_batch_size: DEFAULT_ASSIGN_BATCH_SIZE,
            assign_cache: Arc::new(Mutex::new(None)),
        }
    }
}

impl SuperKMeansIvfClusterer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_centroid_ratio(mut self, centroid_ratio: f32) -> Self {
        self.centroid_ratio = centroid_ratio;
        self
    }

    pub fn with_training_sample_ratio(mut self, training_sample_ratio: f32) -> Self {
        self.training_sample_ratio = training_sample_ratio;
        self
    }

    /// Density-preserving leaf cap: a subsample of size
    /// `training_sample_ratio * N` should still yield about
    /// `centroid_ratio * N` leaves.
    fn max_leaf_size(&self) -> usize {
        let leaf = (self.training_sample_ratio / self.centroid_ratio).round() as usize;
        leaf.max(1)
    }
}

impl IvfClusterer for SuperKMeansIvfClusterer {
    fn training_sample_ratio(&self) -> f32 {
        self.training_sample_ratio
    }

    fn assign_batch_size(&self) -> usize {
        self.assign_batch_size
    }

    fn merge_settings(&self, _total_target_docs: usize) -> tantivy::Result<IvfMergeSettings> {
        let centroid_ratio = self.centroid_ratio;
        let training_sample_ratio = self.training_sample_ratio;
        let assign_batch_size = self.assign_batch_size;

        assert!(
            centroid_ratio > 0.0 && centroid_ratio <= 1.0,
            "centroid_ratio must be in (0, 1], got {centroid_ratio}"
        );
        assert!(
            training_sample_ratio > 0.0 && training_sample_ratio <= 1.0,
            "training_sample_ratio must be in (0, 1], got {training_sample_ratio}"
        );
        assert!(assign_batch_size > 0, "assign_batch_size must be > 0");

        Ok(IvfMergeSettings {
            training_sample_ratio,
            assign_batch_size,
        })
    }

    fn train(
        &self,
        options: &VectorOptions,
        vectors: IvfTrainingVectors,
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
        config.max_leaf_size = self.max_leaf_size();
        if matches!(options.metric(), Metric::Cosine | Metric::Dot) {
            config.base.angular = true;
        }
        let mut clusterer = HierarchicalSuperKMeans::with_config(dim, config);
        let rows = vectors.matrix.rows;
        // Hand the buffer to superkmeans so it can rotate in place instead of
        // keeping a second full-size copy alive through training. Callers
        // (tantivy merge) are responsible for sampling before this call.
        let centroids = clusterer.train_owned(vectors.matrix.values, rows);
        if !centroids.len().is_multiple_of(dim) {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned {} centroid floats, not a multiple of dim {dim}",
                centroids.len()
            )));
        }
        let num_centroids = centroids.len() / dim;
        if num_centroids == 0 {
            return Err(TantivyError::InternalError(
                "SuperKMeans returned zero centroids".to_string(),
            ));
        }
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

        // Build the clusterer once per `(dim, angular)` and reuse it across every batch.
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
                    let clusterer = Arc::new(HierarchicalSuperKMeans::with_config(dim, config));
                    *cache = Some(AssignClusterer {
                        dim,
                        angular,
                        clusterer: clusterer.clone(),
                    });
                    clusterer
                }
            }
        };
        // Primary (nearest-centroid) assignment via superkmeans, angular-aware
        // for cosine/dot. One cluster per vector.
        let primaries = clusterer.assign(
            vector_matrix.values,
            centroid_matrix.values.as_slice(),
            vector_matrix.rows,
        );
        Ok(primaries)
    }
}

/// Select the IVF router on an opened index. Every `Index::open` in pg_search
/// goes through this: tantivy requires a configured router both to build IVF
/// segments at merge time and to open existing ones for search.
pub fn set_ivf_router(index: &mut Index) -> tantivy::Result<()> {
    index.set_ivf_router(IVF_ROUTER)
}

pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_centroid_ratio(options.centroid_ratio())
        .with_training_sample_ratio(options.training_sample_ratio());
    index.set_ivf_clusterer(Arc::new(clusterer));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_leaf_size_is_density_preserving() {
        let clusterer = SuperKMeansIvfClusterer::new()
            .with_centroid_ratio(0.01)
            .with_training_sample_ratio(0.32);
        assert_eq!(clusterer.max_leaf_size(), 32);

        let full = SuperKMeansIvfClusterer::new()
            .with_centroid_ratio(0.01)
            .with_training_sample_ratio(1.0);
        assert_eq!(full.max_leaf_size(), 100);
    }

    #[test]
    fn merge_settings_use_training_sample_ratio() {
        let settings = SuperKMeansIvfClusterer::new()
            .with_training_sample_ratio(0.5)
            .merge_settings(100_000)
            .unwrap();
        assert_eq!(settings.training_sample_ratio, 0.5);
        assert_eq!(settings.assign_batch_size, DEFAULT_ASSIGN_BATCH_SIZE);
    }

    /// The router is fixed per index: setting it twice with the same kind is
    /// idempotent, so opening the same `Index` through several paths is safe.
    #[test]
    fn set_ivf_router_is_idempotent() {
        use tantivy::schema::Schema;

        let mut index = Index::create_in_ram(Schema::builder().build());
        set_ivf_router(&mut index).expect("first set");
        set_ivf_router(&mut index).expect("same kind again");
        assert!(index.set_ivf_router(RouterKind::Rng).is_err());
    }
}
