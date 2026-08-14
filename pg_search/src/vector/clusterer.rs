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

use superkmeans::{
    ClusterTree, HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig, TreeNode,
};
use tantivy::vector::{
    BKTree, BKTreeNode, IvfCentroids, IvfClusterer, IvfMatrix, IvfMergeSettings,
    IvfTrainingVectors, IvfVectors, Metric, VectorOptions,
};
use tantivy::{Index, TantivyError};

use crate::postgres::options::BM25IndexOptions;

const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;

/// SPTAG-style BKT fan-out when clustering IVF centroids for RNG seeding.
const BKT_BRANCHING_FACTOR: usize = 32;
/// Target members per BKT leaf (IVF / RNG node ids).
const BKT_MAX_LEAF_SIZE: usize = 10;

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
    /// Total cells a vector is written into (SPANN `ReplicaCount`). `1` (the
    /// default) is primary-only Phase 1; `> 1` adds up to `replicas - 1`
    /// next-nearest cells at merge time, selected by tantivy's centroid
    /// selector (exact scan or `RelativeNeighborhoodGraph`).
    replicas: usize,
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
            .field("replicas", &self.replicas)
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
            replicas: 1,
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

    pub fn with_replicas(mut self, replicas: usize) -> Self {
        self.replicas = replicas.max(1);
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
            // Replica cells (the `replicas - 1` non-primary cells per vector)
            // are selected by tantivy in the field's raw metric —
            // router-consistent with query-time `rank_centroids`. No angular
            // assumption on this clusterer remains.
            replicas: self.replicas.max(1),
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
                    let clusterer =
                        Arc::new(HierarchicalSuperKMeans::with_config(dim, config));
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
        // for cosine/dot. One cluster per vector — no replication. `n_centroids`
        // is derived from the centroid slice length.
        let primaries = clusterer.assign(
            vector_matrix.values,
            centroid_matrix.values.as_slice(),
            vector_matrix.rows,
        );
        Ok(primaries)
    }

    fn build_bkt(
        &self,
        options: &VectorOptions,
        centroids: &IvfCentroids,
    ) -> tantivy::Result<Option<BKTree<Vec<f32>>>> {
        let IvfCentroids::F32(centroids) = centroids;
        let dim = options.dim();
        let n = centroids.rows;
        if n == 0 {
            return Ok(None);
        }
        if centroids.dims != dim {
            return Err(TantivyError::InvalidArgument(format!(
                "centroid dimensionality mismatch: expected {dim}, got {}",
                centroids.dims
            )));
        }
        if centroids.values.len() != n * dim {
            return Err(TantivyError::InvalidArgument(format!(
                "centroid value count mismatch: expected {}, got {}",
                n * dim,
                centroids.values.len()
            )));
        }
        // Too few centroids for a useful router; `rank_clusters` falls back to
        // strided RNG seeds.
        if n <= BKT_MAX_LEAF_SIZE {
            return Ok(None);
        }

        let mut config = self.config.clone();
        config.branching_factor = Some(BKT_BRANCHING_FACTOR);
        config.max_leaf_size = BKT_MAX_LEAF_SIZE;
        if matches!(options.metric(), Metric::Cosine | Metric::Dot) {
            config.base.angular = true;
        }
        config.base.suppress_warnings = true;

        let mut clusterer = HierarchicalSuperKMeans::with_config(dim, config);
        // Train may run on a sample in the future; membership of every IVF
        // centroid is resolved by `assign` below either way.
        let leaf_centroids = clusterer.train(&centroids.values, n);
        let assignments = clusterer.assign(&centroids.values, &leaf_centroids, n);
        if assignments.len() != n {
            return Err(TantivyError::InternalError(format!(
                "BKT assign returned {} labels, expected {n}",
                assignments.len()
            )));
        }

        let tree = cluster_tree_to_bkt(
            &clusterer.tree,
            &clusterer.base.pruner,
            &assignments,
            dim,
            options.metric(),
        )?;
        Ok(Some(tree))
    }
}

/// Map a SuperKMeans [`ClusterTree`] into a Tantivy [`BKTree`].
///
/// Tree centers are unrotated into the same space as IVF centroids / queries.
/// Leaf `members` are IVF / RNG node ids from `assignments` (one leaf index per
/// centroid row).
fn cluster_tree_to_bkt(
    tree: &ClusterTree,
    pruner: &superkmeans::adsampling::ADSamplingPruner,
    assignments: &[u32],
    dim: usize,
    metric: Metric,
) -> tantivy::Result<BKTree<Vec<f32>>> {
    if tree.is_empty() {
        return Err(TantivyError::InternalError(
            "BKT ClusterTree is empty after training".to_string(),
        ));
    }
    if tree.root.0 != 0 {
        return Err(TantivyError::InternalError(format!(
            "BKT expects ClusterTree root at node 0, got {}",
            tree.root.0
        )));
    }
    if tree.dimensionality() != dim {
        return Err(TantivyError::InternalError(format!(
            "BKT tree dimensionality {} != field dim {dim}",
            tree.dimensionality()
        )));
    }

    let n_nodes = tree.nodes.len();
    let n_leaves = tree.n_leaves;
    if n_leaves == 0 {
        return Err(TantivyError::InternalError(
            "BKT ClusterTree has no leaves".to_string(),
        ));
    }

    // `tree.centroids` are in the ADSampling-rotated train domain; query-time
    // BKT search scores with the field metric against IVF-space vectors.
    let mut centers = vec![0.0_f32; n_nodes * dim];
    pruner.unrotate(&tree.centroids, &mut centers, n_nodes);

    // `assign` labels are indices into `tree.leaves()` order.
    let mut leaf_index_of_node = vec![usize::MAX; n_nodes];
    for (leaf_idx, (node_id, _)) in tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_leaf())
        .enumerate()
    {
        leaf_index_of_node[node_id] = leaf_idx;
    }

    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); n_leaves];
    for (centroid_id, &leaf_idx) in assignments.iter().enumerate() {
        let leaf_idx = leaf_idx as usize;
        if leaf_idx >= n_leaves {
            return Err(TantivyError::InternalError(format!(
                "BKT assign label {leaf_idx} out of range for {n_leaves} leaves"
            )));
        }
        buckets[leaf_idx].push(centroid_id as u32);
    }

    let mut members = Vec::with_capacity(assignments.len());
    let mut leaf_member_range = vec![(0_u32, 0_u32); n_leaves];
    for (leaf_idx, bucket) in buckets.iter().enumerate() {
        let offset = members.len() as u32;
        let size = bucket.len() as u32;
        members.extend_from_slice(bucket);
        leaf_member_range[leaf_idx] = (offset, size);
    }

    let mut nodes = Vec::with_capacity(n_nodes);
    for (node_id, node) in tree.nodes.iter().enumerate() {
        match node {
            TreeNode::Internal {
                centroid_offset,
                children_offset,
                children_size,
                ..
            } => {
                nodes.push(BKTreeNode::Internal {
                    centroid_id: u32::try_from(*centroid_offset).map_err(|_| {
                        TantivyError::InternalError(
                            "BKT centroid_offset exceeds u32".to_string(),
                        )
                    })?,
                    children_offset: u32::try_from(*children_offset).map_err(|_| {
                        TantivyError::InternalError(
                            "BKT children_offset exceeds u32".to_string(),
                        )
                    })?,
                    children_size: u32::try_from(*children_size).map_err(|_| {
                        TantivyError::InternalError("BKT children_size exceeds u32".to_string())
                    })?,
                });
            }
            TreeNode::Leaf {
                centroid_offset, ..
            } => {
                let leaf_idx = leaf_index_of_node[node_id];
                if leaf_idx == usize::MAX {
                    return Err(TantivyError::InternalError(format!(
                        "BKT leaf node {node_id} missing from leaves() enumeration"
                    )));
                }
                let (members_offset, members_size) = leaf_member_range[leaf_idx];
                nodes.push(BKTreeNode::Leaf {
                    centroid_id: u32::try_from(*centroid_offset).map_err(|_| {
                        TantivyError::InternalError(
                            "BKT centroid_offset exceeds u32".to_string(),
                        )
                    })?,
                    members_offset,
                    members_size,
                });
            }
        }
    }

    Ok(BKTree {
        dim,
        metric,
        nodes,
        members,
        centers,
    })
}

pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_centroid_ratio(options.centroid_ratio())
        .with_training_sample_ratio(options.training_sample_ratio())
        .with_replicas(options.cluster_replication());
    index.set_ivf_clusterer(Arc::new(clusterer));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Replication is off by default (`replicas = 1`), and non-positive
    /// configured values clamp to `1` rather than disabling clustering.
    #[test]
    fn replicas_default_and_clamp() {
        let total = 100_000;
        let settings = SuperKMeansIvfClusterer::new()
            .merge_settings(total)
            .unwrap();
        assert_eq!(settings.replicas, 1, "primary-only by default");

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
    fn build_bkt_maps_cluster_tree_members_as_permutation() {
        use tantivy::vector::IvfClusterer;

        let dim = 8;
        let n = 64;
        let mut values = Vec::with_capacity(n * dim);
        for i in 0..n {
            for d in 0..dim {
                values.push((i * dim + d) as f32 * 0.01);
            }
        }
        let centroids = IvfCentroids::F32(IvfMatrix {
            values,
            rows: n,
            dims: dim,
        });
        let options = VectorOptions::new(dim, Metric::L2);

        let tree = SuperKMeansIvfClusterer::new()
            .build_bkt(&options, &centroids)
            .expect("build_bkt")
            .expect("expected Some(BKTree)");

        assert!(!tree.nodes.is_empty());
        assert_eq!(tree.dim, dim);
        assert_eq!(tree.centers.len(), tree.nodes.len() * dim);

        let mut members = tree.members.clone();
        members.sort_unstable();
        let expected: Vec<u32> = (0..n as u32).collect();
        assert_eq!(members, expected, "leaf members must cover every IVF centroid");

        let leaf_count = tree
            .nodes
            .iter()
            .filter(|n| matches!(n, BKTreeNode::Leaf { .. }))
            .count();
        assert!(leaf_count >= 2);
        for node in &tree.nodes {
            if let BKTreeNode::Leaf {
                members_offset,
                members_size,
                ..
            } = node
            {
                assert!(*members_size as usize <= BKT_MAX_LEAF_SIZE + 4);
                let end = *members_offset as usize + *members_size as usize;
                assert!(end <= tree.members.len());
            }
        }
    }

    #[test]
    fn build_bkt_skips_tiny_centroid_sets() {
        use tantivy::vector::IvfClusterer;

        let dim = 4;
        let n = BKT_MAX_LEAF_SIZE;
        let centroids = IvfCentroids::F32(IvfMatrix {
            values: vec![0.0; n * dim],
            rows: n,
            dims: dim,
        });
        let options = VectorOptions::new(dim, Metric::L2);
        let tree = SuperKMeansIvfClusterer::new()
            .build_bkt(&options, &centroids)
            .unwrap();
        assert!(tree.is_none());
    }
}
