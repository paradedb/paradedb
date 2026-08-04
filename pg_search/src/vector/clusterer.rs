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

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};

use superkmeans::{
    ClusterTree, HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig, NodeId, TreeNode,
};
use tantivy::vector::{
    IvfCentroids, IvfClusterer, IvfMatrix, IvfMergeSettings, IvfTrainingVectors, IvfVectors,
    Metric, VectorOptions,
};
use tantivy::{Index, TantivyError};

use crate::postgres::options::BM25IndexOptions;

const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;

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
    training_samples_per_centroid: usize,
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
            .field(
                "training_samples_per_centroid",
                &self.training_samples_per_centroid,
            )
            .field("assign_batch_size", &self.assign_batch_size)
            .field("replicas", &self.replicas)
            .finish_non_exhaustive()
    }
}

impl Default for SuperKMeansIvfClusterer {
    fn default() -> Self {
        // Per-run knobs live on the nested `base` config in superkmeans-rs.
        // `balance_lambda` keeps its default: evenly sized posting lists are
        // the reason we cluster hierarchically in the first place.
        // `branching_factor` and `iters_per_level` keep their defaults, which
        // superkmeans tunes for build cost: the tree stays wide and shallow, so a
        // merge of tens of millions of vectors is only a handful of levels deep.
        let mut config = HierarchicalSuperKMeansConfig::default();
        config.base.suppress_warnings = true;
        config.base.sampling_fraction = 1.0;
        Self {
            config,
            centroid_ratio: 0.01,
            training_samples_per_centroid: 32,
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

        let rows = vectors.matrix.rows;
        let mut config = self.config.clone();
        if matches!(options.metric(), Metric::Cosine | Metric::Dot) {
            config.base.angular = true;
        }
        config.max_leaf_size = max_leaf_size_for(rows, num_centroids);

        let mut clusterer = HierarchicalSuperKMeans::with_config(dim, config);
        // Hand the buffer over so training rotates it in place; at index-build
        // scale a second copy of the sample is the peak-memory term that matters.
        let leaf_centroids = clusterer.train_owned(vectors.matrix.values, rows);

        if leaf_centroids.is_empty() || !leaf_centroids.len().is_multiple_of(dim) {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned {} centroid floats, expected a positive multiple of {dim}",
                leaf_centroids.len()
            )));
        }
        let n_leaves = leaf_centroids.len() / dim;
        if n_leaves != clusterer.tree.n_leaves {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned {n_leaves} centroids for a tree with {} leaves",
                clusterer.tree.n_leaves
            )));
        }

        Ok(IvfCentroids::F32(IvfMatrix {
            values: cut_tree(&clusterer.tree, &leaf_centroids, num_centroids, dim),
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
        // for cosine/dot. One cluster per vector — no replication. `n_centroids`
        // is derived from the centroid slice length.
        let primaries = clusterer.assign(
            vector_matrix.values,
            centroid_matrix.values.as_slice(),
            vector_matrix.rows,
        );
        Ok(primaries)
    }
}

/// Leaf size that makes the tree at least as fine-grained as the requested
/// centroid count. Leaves partition the training sample and hold at most
/// `max_leaf_size` points each, so flooring here guarantees `rows /
/// max_leaf_size >= num_centroids` leaves to cut from — the tree is built
/// finer than we need and [`cut_tree`] stops at the requested count.
fn max_leaf_size_for(rows: usize, num_centroids: usize) -> usize {
    (rows / num_centroids.max(1)).max(1)
}

/// Cut the cluster tree into exactly `num_centroids` cells and return their
/// centroids, row-major.
///
/// Hierarchical balanced clustering no longer takes a cluster count: it
/// returns one centroid per leaf, and the leaf count is emergent. Tantivy's
/// `IvfClusterer` contract still requires exactly the requested number of
/// rows (it sizes the posting-list offsets from that count), so we descend the
/// tree, always expanding the largest cell, until the frontier holds enough
/// cells. Every cut is therefore a balanced partition of the sample.
///
/// A cell's centroid is the size-weighted mean of the leaf centroids beneath
/// it. Rotation is orthogonal and linear, so this stays in whatever domain
/// superkmeans returned its leaf centroids in.
fn cut_tree(
    tree: &ClusterTree,
    leaf_centroids: &[f32],
    num_centroids: usize,
    dim: usize,
) -> Vec<f32> {
    fn push_cell(
        tree: &ClusterTree,
        id: NodeId,
        terminal: &mut Vec<NodeId>,
        splittable: &mut BinaryHeap<(usize, Reverse<usize>)>,
    ) {
        let node = tree.node(id);
        if node.is_leaf() {
            terminal.push(id);
        } else {
            splittable.push((node.size(), Reverse(id.0)));
        }
    }

    let mut terminal: Vec<NodeId> = Vec::with_capacity(num_centroids);
    let mut splittable: BinaryHeap<(usize, Reverse<usize>)> = BinaryHeap::new();
    push_cell(tree, tree.root, &mut terminal, &mut splittable);

    while terminal.len() + splittable.len() < num_centroids {
        let Some((_, Reverse(id))) = splittable.pop() else {
            break;
        };
        for &child in tree.node(NodeId(id)).children() {
            push_cell(tree, child, &mut terminal, &mut splittable);
        }
    }

    // The last expansion overshoots by up to `branching_factor - 1` cells;
    // dropping the smallest of them costs less recall than dropping large
    // ones, and their points fall through to the nearest surviving centroid.
    let mut cells: Vec<NodeId> = terminal
        .into_iter()
        .chain(splittable.into_iter().map(|(_, Reverse(id))| NodeId(id)))
        .collect();
    cells.sort_unstable_by_key(|id| (Reverse(tree.node(*id).size()), id.0));
    cells.truncate(num_centroids);

    let mut values = Vec::with_capacity(num_centroids * dim);
    for &id in &cells {
        values.extend(cell_centroid(tree, id, leaf_centroids, dim));
    }
    // Only reachable when the sample carries fewer points than the requested
    // centroid count, which leaves the tree short of cells to cut. Repeating
    // centroids keeps tantivy's row-count contract; the duplicates lose every
    // nearest-centroid tie and stay empty.
    while values.len() < num_centroids * dim {
        values.extend_from_within(0..dim);
    }
    values
}

fn cell_centroid(tree: &ClusterTree, cell: NodeId, leaf_centroids: &[f32], dim: usize) -> Vec<f32> {
    let mut sum = vec![0.0_f64; dim];
    let mut members = 0_usize;
    let mut stack = vec![cell];
    while let Some(id) = stack.pop() {
        match tree.node(id) {
            TreeNode::Leaf { leaf_id, size, .. } => {
                let start = leaf_id.0 * dim;
                let leaf = &leaf_centroids[start..start + dim];
                for (acc, value) in sum.iter_mut().zip(leaf) {
                    *acc += f64::from(*value) * *size as f64;
                }
                members += size;
            }
            TreeNode::Internal { children, .. } => stack.extend(children.iter().copied()),
        }
    }
    let scale = if members == 0 {
        0.0
    } else {
        1.0 / members as f64
    };
    sum.into_iter().map(|acc| (acc * scale) as f32).collect()
}

pub fn set_ivf_clusterer(index: &mut Index, options: &BM25IndexOptions) {
    let clusterer = SuperKMeansIvfClusterer::new()
        .with_centroid_ratio(options.centroid_ratio())
        .with_training_samples_per_centroid(options.training_samples_per_centroid())
        .with_replicas(options.cluster_replication());
    index.set_ivf_clusterer(Arc::new(clusterer));
}

#[cfg(test)]
mod tests {
    use super::*;
    use superkmeans::LeafId;

    /// Two-level binary tree over 8 points: the root splits into a heavy left
    /// child (two leaves of 3 and 3) and a light right leaf of 2.
    fn sample_tree() -> (ClusterTree, Vec<f32>) {
        let leaf_centroids = vec![0.0, 0.0, 6.0, 6.0, 30.0, 30.0];
        let tree = ClusterTree {
            nodes: vec![
                TreeNode::Internal {
                    centroid_offset: 0,
                    size: 8,
                    children: vec![NodeId(1), NodeId(2)],
                },
                TreeNode::Internal {
                    centroid_offset: 1,
                    size: 6,
                    children: vec![NodeId(3), NodeId(4)],
                },
                TreeNode::Leaf {
                    centroid_offset: 2,
                    size: 2,
                    leaf_id: LeafId(2),
                },
                TreeNode::Leaf {
                    centroid_offset: 3,
                    size: 3,
                    leaf_id: LeafId(0),
                },
                TreeNode::Leaf {
                    centroid_offset: 4,
                    size: 3,
                    leaf_id: LeafId(1),
                },
            ],
            centroids: Vec::new(),
            leaf_members: vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7]],
            n_leaves: 3,
            root: NodeId(0),
        };
        (tree, leaf_centroids)
    }

    /// Flooring keeps the tree finer than the requested cut, so there is
    /// always something left to expand.
    #[test]
    fn max_leaf_size_bounds_the_leaf_count() {
        assert_eq!(max_leaf_size_for(1_000, 10), 100);
        assert_eq!(max_leaf_size_for(1_050, 10), 105);
        assert_eq!(max_leaf_size_for(5, 10), 1, "more centroids than rows");
        assert_eq!(max_leaf_size_for(0, 0), 1, "no division by zero");
        for (rows, requested) in [(1_000usize, 7usize), (999, 128), (64, 64), (10_000, 3)] {
            let leaves = rows.div_ceil(max_leaf_size_for(rows, requested));
            assert!(
                leaves >= requested.min(rows),
                "{rows} rows / {requested} centroids yields only {leaves} leaves"
            );
        }
    }

    /// The cut returns exactly what was requested at every depth, and an
    /// internal cell carries the size-weighted mean of the leaves under it.
    #[test]
    fn cut_tree_returns_requested_rows() {
        let (tree, leaf_centroids) = sample_tree();

        let root_only = cut_tree(&tree, &leaf_centroids, 1, 2);
        assert_eq!(root_only, vec![9.75, 9.75], "(3·0 + 3·6 + 2·30) / 8");

        let split = cut_tree(&tree, &leaf_centroids, 2, 2);
        assert_eq!(split, vec![3.0, 3.0, 30.0, 30.0], "largest cell first");

        let all_leaves = cut_tree(&tree, &leaf_centroids, 3, 2);
        assert_eq!(all_leaves, vec![0.0, 0.0, 6.0, 6.0, 30.0, 30.0]);
    }

    /// A tree with fewer cells than centroids still fills the matrix, because
    /// tantivy rejects any row count other than the one it asked for.
    #[test]
    fn cut_tree_pads_when_the_tree_runs_out() {
        let (tree, leaf_centroids) = sample_tree();
        let padded = cut_tree(&tree, &leaf_centroids, 5, 2);
        assert_eq!(padded.len(), 10);
        assert_eq!(&padded[..6], &[0.0, 0.0, 6.0, 6.0, 30.0, 30.0]);
        assert_eq!(&padded[6..], &[0.0, 0.0, 0.0, 0.0]);
    }

    /// End-to-end over a real tree: training with the derived `max_leaf_size`
    /// always leaves more cells than the cut asks for, and the cut hands back
    /// exactly the row count tantivy demands.
    #[test]
    fn training_produces_enough_cells_to_cut() {
        let (rows, dim, requested) = (2_000usize, 8usize, 50usize);
        let data = superkmeans::make_blobs(rows, dim, 12, true, 1.0, 5.0, 42);

        let mut config = SuperKMeansIvfClusterer::new().config;
        config.max_leaf_size = max_leaf_size_for(rows, requested);
        let mut clusterer = HierarchicalSuperKMeans::with_config(dim, config);
        let leaf_centroids = clusterer.train(&data, rows);

        assert!(
            leaf_centroids.len() / dim >= requested,
            "{} leaves for {requested} requested centroids",
            leaf_centroids.len() / dim
        );
        let centroids = cut_tree(&clusterer.tree, &leaf_centroids, requested, dim);
        assert_eq!(centroids.len(), requested * dim);
        assert!(centroids.iter().all(|value| value.is_finite()));
    }

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
}
