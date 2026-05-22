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

use std::sync::Arc;

use superkmeans::{HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig, SuperKMeansError};
use tantivy::vector::{IvfCentroids, IvfClusterer, IvfMatrix, IvfVectors, Metric, VectorOptions};
use tantivy::{Index, TantivyError};

#[cfg(target_os = "macos")]
const DEFAULT_ASSIGN_BATCH_SIZE: usize = 40_960;
#[cfg(not(target_os = "macos"))]
const DEFAULT_ASSIGN_BATCH_SIZE: usize = 4_096;

#[derive(Clone, Debug)]
pub struct SuperKMeansIvfClusterer {
    config: HierarchicalSuperKMeansConfig,
    centroid_ratio: f32,
    training_samples_per_centroid: usize,
    assign_batch_size: usize,
}

impl Default for SuperKMeansIvfClusterer {
    fn default() -> Self {
        let config = HierarchicalSuperKMeansConfig {
            suppress_warnings: true,
            sampling_fraction: 1.0,
            ..Default::default()
        };
        Self {
            config,
            centroid_ratio: 0.01,
            training_samples_per_centroid: 32,
            assign_batch_size: DEFAULT_ASSIGN_BATCH_SIZE,
        }
    }
}

impl SuperKMeansIvfClusterer {
    pub fn new() -> Self {
        Self::default()
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

    fn train(
        &self,
        options: &VectorOptions,
        vectors: IvfVectors<'_>,
        num_centroids: usize,
    ) -> tantivy::Result<IvfCentroids> {
        let IvfVectors::F32(vectors) = vectors;
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
            config.angular = true;
        }
        let mut clusterer = HierarchicalSuperKMeans::with_config(num_centroids, dim, config)
            .map_err(to_tantivy_error)?;
        let centroids = clusterer
            .train(vectors.matrix.values, vectors.matrix.rows)
            .map_err(to_tantivy_error)?;
        if centroids.len() != num_centroids * dim {
            return Err(TantivyError::InternalError(format!(
                "SuperKMeans returned {} centroid floats, expected {}",
                centroids.len(),
                num_centroids * dim
            )));
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

        let mut config = self.config.clone();
        if matches!(options.metric(), Metric::Cosine | Metric::Dot) {
            config.angular = true;
        }
        let mut clusterer = HierarchicalSuperKMeans::with_config(centroid_matrix.rows, dim, config)
            .map_err(to_tantivy_error)?;
        clusterer
            .assign(
                vector_matrix.values,
                centroid_matrix.values.as_slice(),
                vector_matrix.rows,
                centroid_matrix.rows,
            )
            .map_err(to_tantivy_error)
    }
}

pub fn set_ivf_clusterer(index: &mut Index) {
    index.set_ivf_clusterer(Arc::new(SuperKMeansIvfClusterer::new()));
}

fn to_tantivy_error(error: SuperKMeansError) -> TantivyError {
    match error {
        SuperKMeansError::InvalidArgument(message) => TantivyError::InvalidArgument(message),
        SuperKMeansError::Runtime(message) => TantivyError::InternalError(message),
    }
}
