use crate::common::{
    ensure_positive_usize, ClusterBalanceStats, Result, SuperKMeansError, SuperKMeansIterationStats,
};
use crate::ffi::HierarchicalHandle;

#[derive(Debug, Clone)]
pub struct HierarchicalSuperKMeansConfig {
    pub iters_mesoclustering: usize,
    pub iters_fineclustering: usize,
    pub iters_refinement: usize,
    pub iters: usize,
    pub sampling_fraction: f32,
    pub max_points_per_cluster: usize,
    pub n_threads: usize,
    pub seed: u32,
    pub use_blas_only: bool,
    pub tol: f32,
    pub recall_tol: f32,
    pub early_termination: bool,
    pub sample_queries: bool,
    pub objective_k: usize,
    pub ann_explore_fraction: f32,
    pub min_not_pruned_pct: f32,
    pub max_not_pruned_pct: f32,
    pub adjustment_factor_for_partial_d: f32,
    pub unrotate_centroids: bool,
    pub verbose: bool,
    pub angular: bool,
    pub suppress_warnings: bool,
    pub data_already_rotated: bool,
}

impl Default for HierarchicalSuperKMeansConfig {
    fn default() -> Self {
        Self {
            iters_mesoclustering: 3,
            iters_fineclustering: 5,
            iters_refinement: 0,
            iters: 10,
            sampling_fraction: 1.0,
            max_points_per_cluster: 256,
            n_threads: 0,
            seed: 42,
            use_blas_only: false,
            tol: 1e-4,
            recall_tol: 0.005,
            early_termination: true,
            sample_queries: false,
            objective_k: 100,
            ann_explore_fraction: 0.01,
            min_not_pruned_pct: 0.03,
            max_not_pruned_pct: 0.05,
            adjustment_factor_for_partial_d: 0.20,
            unrotate_centroids: true,
            verbose: false,
            angular: false,
            suppress_warnings: false,
            data_already_rotated: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HierarchicalSuperKMeansIterationStats {
    pub mesoclustering_iteration_stats: Vec<SuperKMeansIterationStats>,
    pub refinement_iteration_stats: Vec<SuperKMeansIterationStats>,
    pub fineclustering_iteration_stats: Vec<SuperKMeansIterationStats>,
}

pub struct HierarchicalSuperKMeans {
    pub iteration_stats: Vec<SuperKMeansIterationStats>,
    pub hierarchical_config: HierarchicalSuperKMeansConfig,
    pub hierarchical_iteration_stats: HierarchicalSuperKMeansIterationStats,
    dimensionality: usize,
    cpp: HierarchicalHandle,
    distances_len: usize,
}

impl HierarchicalSuperKMeans {
    pub fn new(n_clusters: usize, dimensionality: usize) -> Result<Self> {
        Self::with_config(
            n_clusters,
            dimensionality,
            HierarchicalSuperKMeansConfig::default(),
        )
    }

    pub fn with_config(
        n_clusters: usize,
        dimensionality: usize,
        config: HierarchicalSuperKMeansConfig,
    ) -> Result<Self> {
        validate_config_widths(&config)?;
        let cpp = HierarchicalHandle::new(n_clusters, dimensionality, &config)?;
        let config = cpp.config();
        Ok(Self {
            iteration_stats: Vec::new(),
            hierarchical_config: config,
            hierarchical_iteration_stats: HierarchicalSuperKMeansIterationStats::default(),
            dimensionality,
            cpp,
            distances_len: 0,
        })
    }

    pub fn train(&mut self, data: &[f32], n: usize) -> Result<Vec<f32>> {
        self.train_with_queries(data, n, None, 0)
    }

    pub fn train_with_queries(
        &mut self,
        data: &[f32],
        n: usize,
        queries: Option<&[f32]>,
        n_queries: usize,
    ) -> Result<Vec<f32>> {
        ensure_positive_usize(n, "n")?;
        validate_matrix_len(data, n, self.dimensionality, "data")?;
        let centroids = self.cpp.train(data, n, queries, n_queries)?;
        self.refresh_after_training(n);
        Ok(centroids)
    }

    pub fn assign(
        &mut self,
        vectors: &[f32],
        centroids: &[f32],
        n_vectors: usize,
        n_centroids: usize,
    ) -> Result<Vec<u32>> {
        validate_matrix_len(vectors, n_vectors, self.dimensionality, "vectors")?;
        validate_centroids_len(centroids, n_centroids, self.dimensionality)?;
        self.cpp.assign(vectors, centroids, n_vectors, n_centroids)
    }

    pub fn assign_training_points(
        &mut self,
        vectors: &[f32],
        centroids: &[f32],
        n_vectors: usize,
        n_centroids: usize,
    ) -> Result<Vec<u32>> {
        validate_matrix_len(vectors, n_vectors, self.dimensionality, "vectors")?;
        validate_centroids_len(centroids, n_centroids, self.dimensionality)?;
        let assignments =
            self.cpp
                .assign_training_points(vectors, centroids, n_vectors, n_centroids)?;
        self.distances_len = n_vectors;
        Ok(assignments)
    }

    pub fn is_trained(&self) -> bool {
        self.cpp.is_trained()
    }

    pub fn get_n_clusters(&self) -> usize {
        self.cpp.n_clusters()
    }

    pub fn get_sampling_fraction(&self) -> f32 {
        self.cpp.sampling_fraction()
    }

    pub fn get_distances(&self) -> &[f32] {
        self.cpp.distances(self.distances_len)
    }

    pub fn get_clusters_balance_stats(
        assignments: &[u32],
        n_samples: usize,
        n_clusters: usize,
    ) -> ClusterBalanceStats {
        HierarchicalHandle::clusters_balance_stats(assignments, n_samples, n_clusters)
    }

    pub fn get_n_mesoclusters(n_clusters: usize) -> usize {
        HierarchicalHandle::get_n_mesoclusters(n_clusters)
    }

    pub fn get_n_vectors_to_sample(&self, n: usize, n_clusters: usize) -> usize {
        self.cpp.get_n_vectors_to_sample(n, n_clusters)
    }

    fn refresh_after_training(&mut self, n: usize) {
        self.hierarchical_iteration_stats
            .mesoclustering_iteration_stats = self.cpp.mesostats();
        self.hierarchical_iteration_stats
            .fineclustering_iteration_stats = self.cpp.finestats();
        self.hierarchical_iteration_stats.refinement_iteration_stats = self.cpp.refinestats();
        self.iteration_stats.clear();
        self.distances_len = n;
    }
}

fn validate_matrix_len(data: &[f32], n: usize, d: usize, name: &str) -> Result<()> {
    if data.len() < n * d {
        return Err(SuperKMeansError::InvalidArgument(format!(
            "{name} length is smaller than n * dimensionality"
        )));
    }
    Ok(())
}

fn validate_centroids_len(centroids: &[f32], n_centroids: usize, d: usize) -> Result<()> {
    if centroids.len() < n_centroids * d {
        return Err(SuperKMeansError::InvalidArgument(
            "centroids length is smaller than n_centroids * dimensionality".to_string(),
        ));
    }
    Ok(())
}

fn validate_config_widths(config: &HierarchicalSuperKMeansConfig) -> Result<()> {
    validate_u32(config.iters, "config.iters")?;
    validate_u32(
        config.max_points_per_cluster,
        "config.max_points_per_cluster",
    )?;
    validate_u32(config.n_threads, "config.n_threads")?;
    validate_u32(config.iters_mesoclustering, "config.iters_mesoclustering")?;
    validate_u32(config.iters_fineclustering, "config.iters_fineclustering")?;
    validate_u32(config.iters_refinement, "config.iters_refinement")?;
    Ok(())
}

fn validate_u32(value: usize, name: &str) -> Result<()> {
    if value > u32::MAX as usize {
        return Err(SuperKMeansError::InvalidArgument(format!(
            "{name} must fit into uint32_t"
        )));
    }
    Ok(())
}
