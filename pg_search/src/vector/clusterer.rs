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
    BuiltRouter, IvfCentroids, IvfClusterer, IvfConfig, IvfIndexBuilder, IvfMatrix,
    IvfMergeSettings, IvfTrainingVectors, IvfVectors, Metric, NeighborhoodGraphConfig,
    RelativeNeighborhoodGraph, SuperKMeansLevelClusterer, VectorOptions,
};
use tantivy::{Executor, Index, TantivyError};

use crate::gucs::{self, VectorRouter};
use crate::postgres::options::BM25IndexOptions;

const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;
/// Stacked-IVF branching factor for the routing index over trained centroids.
const ROUTER_BRANCHING_FACTOR: usize = 16;
const ROUTER_ITERS_PER_SPLIT: u32 = 5;

fn router_build_executor() -> tantivy::Result<Executor> {
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    if num_threads > 1 {
        Executor::multi_thread(num_threads, "ivf-router-")
    } else {
        Ok(Executor::single_thread())
    }
}

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
    router: VectorRouter,
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
            .field("router", &self.router)
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
            router: VectorRouter::Graph,
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

    pub fn with_router(mut self, router: VectorRouter) -> Self {
        self.router = router;
        self
    }

    /// Density-preserving leaf cap: a subsample of size
    /// `training_sample_ratio * N` should still yield about
    /// `centroid_ratio * N` leaves.
    fn max_leaf_size(&self) -> usize {
        let leaf = (self.training_sample_ratio / self.centroid_ratio).round() as usize;
        leaf.max(1)
    }

    fn build_stacked_router(
        &self,
        options: &VectorOptions,
        matrix: &IvfMatrix<f32>,
    ) -> tantivy::Result<BuiltRouter> {
        let clusterer = SuperKMeansLevelClusterer {
            iters_per_split: ROUTER_ITERS_PER_SPLIT,
        };
        let (index, perm) = IvfIndexBuilder::new(
            matrix.values.clone(),
            matrix.rows,
            options.dim(),
            &clusterer,
            IvfConfig::new(ROUTER_BRANCHING_FACTOR),
        )
        .build();
        Ok(BuiltRouter::Stacked { index, perm })
    }

    fn build_graph_router(
        &self,
        options: &VectorOptions,
        matrix: &IvfMatrix<f32>,
    ) -> tantivy::Result<BuiltRouter> {
        // Build needs a borrowed arena; BuiltRouter::Graph needs owned vectors.
        // Serialize adjacency after build and reopen over the owned buffer.
        let vectors = matrix.values.clone();
        let config = NeighborhoodGraphConfig::default();
        let mut borrowed = RelativeNeighborhoodGraph::new(
            vectors.as_slice(),
            options.dim(),
            options.metric(),
            config,
        );
        borrowed.build(&router_build_executor()?);
        let mut adjacency = Vec::new();
        borrowed.serialize(&mut adjacency)?;
        let owned = RelativeNeighborhoodGraph::open(
            &adjacency,
            vectors,
            options.dim(),
            options.metric(),
            config,
        )?;
        Ok(BuiltRouter::Graph(owned))
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

    fn build_router(
        &self,
        options: &VectorOptions,
        centroids: &IvfCentroids,
    ) -> tantivy::Result<Option<BuiltRouter>> {
        let IvfCentroids::F32(matrix) = centroids;
        if matrix.rows <= 1 {
            return Ok(None);
        }

        let router = match self.router {
            VectorRouter::Ivf => self.build_stacked_router(options, matrix)?,
            VectorRouter::Graph => self.build_graph_router(options, matrix)?,
        };
        Ok(Some(router))
    }
}

pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_centroid_ratio(options.centroid_ratio())
        .with_training_sample_ratio(options.training_sample_ratio())
        .with_router(gucs::vector_router());
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

    fn sample_centroids(dim: usize, n: usize) -> IvfCentroids {
        let mut values = Vec::with_capacity(n * dim);
        for i in 0..n {
            for d in 0..dim {
                values.push((i * dim + d) as f32 * 0.01);
            }
        }
        IvfCentroids::F32(IvfMatrix {
            values,
            rows: n,
            dims: dim,
        })
    }

    #[test]
    fn build_router_returns_stacked_for_ivf() {
        use tantivy::vector::IvfClusterer;

        let dim = 8;
        let n = 64;
        let centroids = sample_centroids(dim, n);
        let options = VectorOptions::new(dim, Metric::L2);

        let router = SuperKMeansIvfClusterer::new()
            .with_router(VectorRouter::Ivf)
            .build_router(&options, &centroids)
            .expect("build_router")
            .expect("expected Some(BuiltRouter)");

        match router {
            BuiltRouter::Stacked { index, perm } => {
                assert_eq!(perm.len(), n);
                assert!(index.nlist() > 0);
            }
            BuiltRouter::Graph(_) => panic!("expected stacked router, not graph"),
        }
    }

    #[test]
    fn build_router_returns_graph_when_configured() {
        use tantivy::vector::IvfClusterer;

        let dim = 8;
        let n = 32;
        let centroids = sample_centroids(dim, n);
        let options = VectorOptions::new(dim, Metric::L2);

        let router = SuperKMeansIvfClusterer::new()
            .with_router(VectorRouter::Graph)
            .build_router(&options, &centroids)
            .expect("build_router")
            .expect("expected Some(BuiltRouter)");

        match router {
            BuiltRouter::Graph(graph) => assert_eq!(graph.len(), n),
            BuiltRouter::Stacked { .. } => panic!("expected graph router, not stacked"),
        }
    }

    #[test]
    fn build_router_skips_single_centroid() {
        use tantivy::vector::IvfClusterer;

        let dim = 4;
        let centroids = IvfCentroids::F32(IvfMatrix {
            values: vec![0.0; dim],
            rows: 1,
            dims: dim,
        });
        let options = VectorOptions::new(dim, Metric::L2);
        let router = SuperKMeansIvfClusterer::new()
            .with_router(VectorRouter::Ivf)
            .build_router(&options, &centroids)
            .unwrap();
        assert!(router.is_none());
    }
}
