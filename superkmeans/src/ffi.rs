use crate::common::{ClusterBalanceStats, Result, SuperKMeansError, SuperKMeansIterationStats};
use crate::hierarchical::HierarchicalSuperKMeansConfig;
use std::ffi::{c_char, c_void, CStr};
use std::marker::PhantomData;
use std::ptr::NonNull;

#[repr(C)]
struct SkmHierConfig {
    iters: u32,
    sampling_fraction: f32,
    max_points_per_cluster: u32,
    n_threads: u32,
    seed: u32,
    use_blas_only: u8,
    tol: f32,
    recall_tol: f32,
    early_termination: u8,
    sample_queries: u8,
    objective_k: usize,
    ann_explore_fraction: f32,
    min_not_pruned_pct: f32,
    max_not_pruned_pct: f32,
    adjustment_factor_for_partial_d: f32,
    unrotate_centroids: u8,
    verbose: u8,
    angular: u8,
    suppress_warnings: u8,
    data_already_rotated: u8,
    iters_mesoclustering: u32,
    iters_fineclustering: u32,
    iters_refinement: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SkmIterationStats {
    iteration: usize,
    objective: f32,
    shift: f32,
    split: usize,
    recall: f32,
    not_pruned_pct: f32,
    partial_d: u32,
    is_gemm_only: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SkmClusterBalanceStats {
    mean: f32,
    geometric_mean: f32,
    stdev: f32,
    cv: f32,
    min: usize,
    max: usize,
}

enum SkmHierHandle {}

unsafe extern "C" {
    fn skm_string_free(value: *mut c_char);
    fn skm_buffer_free(value: *mut c_void);
    fn skm_hier_new(
        n_clusters: usize,
        dimensionality: usize,
        config: *const SkmHierConfig,
        out_error: *mut *mut c_char,
    ) -> *mut SkmHierHandle;
    fn skm_hier_free(handle: *mut SkmHierHandle);
    fn skm_hier_train(
        handle: *mut SkmHierHandle,
        data: *const f32,
        n: usize,
        queries: *const f32,
        n_queries: usize,
        out_centroids: *mut *mut f32,
        out_len: *mut usize,
    ) -> i32;
    fn skm_hier_assign(
        handle: *mut SkmHierHandle,
        vectors: *const f32,
        centroids: *const f32,
        n_vectors: usize,
        n_centroids: usize,
        out_assignments: *mut *mut u32,
        out_len: *mut usize,
    ) -> i32;
    fn skm_hier_assign_training_points(
        handle: *mut SkmHierHandle,
        vectors: *const f32,
        centroids: *const f32,
        n_vectors: usize,
        n_centroids: usize,
        out_assignments: *mut *mut u32,
        out_len: *mut usize,
    ) -> i32;
    fn skm_hier_last_error(handle: *const SkmHierHandle) -> *const c_char;
    fn skm_hier_n_clusters(handle: *const SkmHierHandle) -> usize;
    fn skm_hier_is_trained(handle: *const SkmHierHandle) -> u8;
    fn skm_hier_sampling_fraction(handle: *const SkmHierHandle) -> f32;
    fn skm_hier_copy_config(handle: *const SkmHierHandle, out: *mut SkmHierConfig);
    fn skm_hier_distances(handle: *const SkmHierHandle) -> *const f32;
    fn skm_hier_mesostats_len(handle: *const SkmHierHandle) -> usize;
    fn skm_hier_finestats_len(handle: *const SkmHierHandle) -> usize;
    fn skm_hier_refinestats_len(handle: *const SkmHierHandle) -> usize;
    fn skm_hier_copy_mesostats(handle: *const SkmHierHandle, out: *mut SkmIterationStats);
    fn skm_hier_copy_finestats(handle: *const SkmHierHandle, out: *mut SkmIterationStats);
    fn skm_hier_copy_refinestats(handle: *const SkmHierHandle, out: *mut SkmIterationStats);
    fn skm_hier_get_n_mesoclusters(n_clusters: usize) -> usize;
    fn skm_hier_get_n_vectors_to_sample(
        handle: *const SkmHierHandle,
        n: usize,
        n_clusters: usize,
    ) -> usize;
    fn skm_hier_get_clusters_balance_stats(
        assignments: *const u32,
        n_samples: usize,
        n_clusters: usize,
    ) -> SkmClusterBalanceStats;
}

pub struct HierarchicalHandle {
    raw: NonNull<SkmHierHandle>,
    _not_send_sync: PhantomData<*mut ()>,
}

impl HierarchicalHandle {
    pub fn new(
        n_clusters: usize,
        dimensionality: usize,
        config: &HierarchicalSuperKMeansConfig,
    ) -> Result<Self> {
        let c_config = to_c_config(config)?;
        let mut error = std::ptr::null_mut();
        let raw = unsafe { skm_hier_new(n_clusters, dimensionality, &c_config, &mut error) };
        if let Some(raw) = NonNull::new(raw) {
            return Ok(Self {
                raw,
                _not_send_sync: PhantomData,
            });
        }
        Err(error_from_owned_c_string(error))
    }

    pub fn train(
        &mut self,
        data: &[f32],
        n: usize,
        queries: Option<&[f32]>,
        n_queries: usize,
    ) -> Result<Vec<f32>> {
        let mut out = std::ptr::null_mut();
        let mut len = 0_usize;
        let query_ptr = queries.map_or(std::ptr::null(), |values| values.as_ptr());
        let rc = unsafe {
            skm_hier_train(
                self.raw.as_ptr(),
                data.as_ptr(),
                n,
                query_ptr,
                n_queries,
                &mut out,
                &mut len,
            )
        };
        if rc != 0 {
            return Err(self.last_error());
        }
        copy_owned_buffer(out, len)
    }

    pub fn assign(
        &mut self,
        vectors: &[f32],
        centroids: &[f32],
        n_vectors: usize,
        n_centroids: usize,
    ) -> Result<Vec<u32>> {
        let mut out = std::ptr::null_mut();
        let mut len = 0_usize;
        let rc = unsafe {
            skm_hier_assign(
                self.raw.as_ptr(),
                vectors.as_ptr(),
                centroids.as_ptr(),
                n_vectors,
                n_centroids,
                &mut out,
                &mut len,
            )
        };
        if rc != 0 {
            return Err(self.last_error());
        }
        copy_owned_buffer(out, len)
    }

    pub fn assign_training_points(
        &mut self,
        vectors: &[f32],
        centroids: &[f32],
        n_vectors: usize,
        n_centroids: usize,
    ) -> Result<Vec<u32>> {
        let mut out = std::ptr::null_mut();
        let mut len = 0_usize;
        let rc = unsafe {
            skm_hier_assign_training_points(
                self.raw.as_ptr(),
                vectors.as_ptr(),
                centroids.as_ptr(),
                n_vectors,
                n_centroids,
                &mut out,
                &mut len,
            )
        };
        if rc != 0 {
            return Err(self.last_error());
        }
        copy_owned_buffer(out, len)
    }

    pub fn n_clusters(&self) -> usize {
        unsafe { skm_hier_n_clusters(self.raw.as_ptr()) }
    }

    pub fn is_trained(&self) -> bool {
        unsafe { skm_hier_is_trained(self.raw.as_ptr()) != 0 }
    }

    pub fn sampling_fraction(&self) -> f32 {
        unsafe { skm_hier_sampling_fraction(self.raw.as_ptr()) }
    }

    pub fn config(&self) -> HierarchicalSuperKMeansConfig {
        let mut config = SkmHierConfig::default();
        unsafe { skm_hier_copy_config(self.raw.as_ptr(), &mut config) };
        config.into()
    }

    pub fn distances(&self, len: usize) -> &[f32] {
        let ptr = unsafe { skm_hier_distances(self.raw.as_ptr()) };
        if ptr.is_null() || len == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    pub fn mesostats(&self) -> Vec<SuperKMeansIterationStats> {
        let len = unsafe { skm_hier_mesostats_len(self.raw.as_ptr()) };
        self.copy_stats(len, skm_hier_copy_mesostats)
    }

    pub fn finestats(&self) -> Vec<SuperKMeansIterationStats> {
        let len = unsafe { skm_hier_finestats_len(self.raw.as_ptr()) };
        self.copy_stats(len, skm_hier_copy_finestats)
    }

    pub fn refinestats(&self) -> Vec<SuperKMeansIterationStats> {
        let len = unsafe { skm_hier_refinestats_len(self.raw.as_ptr()) };
        self.copy_stats(len, skm_hier_copy_refinestats)
    }

    pub fn get_n_mesoclusters(n_clusters: usize) -> usize {
        unsafe { skm_hier_get_n_mesoclusters(n_clusters) }
    }

    pub fn get_n_vectors_to_sample(&self, n: usize, n_clusters: usize) -> usize {
        unsafe { skm_hier_get_n_vectors_to_sample(self.raw.as_ptr(), n, n_clusters) }
    }

    pub fn clusters_balance_stats(
        assignments: &[u32],
        n_samples: usize,
        n_clusters: usize,
    ) -> ClusterBalanceStats {
        assert!(assignments.len() >= n_samples);
        unsafe { skm_hier_get_clusters_balance_stats(assignments.as_ptr(), n_samples, n_clusters) }
            .into()
    }

    fn copy_stats(
        &self,
        len: usize,
        copy_fn: unsafe extern "C" fn(*const SkmHierHandle, *mut SkmIterationStats),
    ) -> Vec<SuperKMeansIterationStats> {
        let mut stats = vec![SkmIterationStats::default(); len];
        unsafe { copy_fn(self.raw.as_ptr(), stats.as_mut_ptr()) };
        stats.into_iter().map(Into::into).collect()
    }

    fn last_error(&self) -> SuperKMeansError {
        let ptr = unsafe { skm_hier_last_error(self.raw.as_ptr()) };
        if ptr.is_null() {
            return SuperKMeansError::Runtime("unknown C++ exception".to_string());
        }
        let message = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        SuperKMeansError::Runtime(message)
    }
}

impl Drop for HierarchicalHandle {
    fn drop(&mut self) {
        unsafe { skm_hier_free(self.raw.as_ptr()) };
    }
}

impl Default for SkmIterationStats {
    fn default() -> Self {
        Self {
            iteration: 0,
            objective: 0.0,
            shift: 0.0,
            split: 0,
            recall: 0.0,
            not_pruned_pct: -1.0,
            partial_d: 0,
            is_gemm_only: 0,
        }
    }
}

impl Default for SkmHierConfig {
    fn default() -> Self {
        Self {
            iters: 0,
            sampling_fraction: 0.0,
            max_points_per_cluster: 0,
            n_threads: 0,
            seed: 0,
            use_blas_only: 0,
            tol: 0.0,
            recall_tol: 0.0,
            early_termination: 0,
            sample_queries: 0,
            objective_k: 0,
            ann_explore_fraction: 0.0,
            min_not_pruned_pct: 0.0,
            max_not_pruned_pct: 0.0,
            adjustment_factor_for_partial_d: 0.0,
            unrotate_centroids: 0,
            verbose: 0,
            angular: 0,
            suppress_warnings: 0,
            data_already_rotated: 0,
            iters_mesoclustering: 0,
            iters_fineclustering: 0,
            iters_refinement: 0,
        }
    }
}

impl From<SkmIterationStats> for SuperKMeansIterationStats {
    fn from(value: SkmIterationStats) -> Self {
        Self {
            iteration: value.iteration,
            objective: value.objective,
            shift: value.shift,
            split: value.split,
            recall: value.recall,
            not_pruned_pct: value.not_pruned_pct,
            partial_d: value.partial_d as usize,
            is_gemm_only: value.is_gemm_only != 0,
        }
    }
}

impl From<SkmHierConfig> for HierarchicalSuperKMeansConfig {
    fn from(value: SkmHierConfig) -> Self {
        Self {
            iters_mesoclustering: value.iters_mesoclustering as usize,
            iters_fineclustering: value.iters_fineclustering as usize,
            iters_refinement: value.iters_refinement as usize,
            iters: value.iters as usize,
            sampling_fraction: value.sampling_fraction,
            max_points_per_cluster: value.max_points_per_cluster as usize,
            n_threads: value.n_threads as usize,
            seed: value.seed,
            use_blas_only: value.use_blas_only != 0,
            tol: value.tol,
            recall_tol: value.recall_tol,
            early_termination: value.early_termination != 0,
            sample_queries: value.sample_queries != 0,
            objective_k: value.objective_k,
            ann_explore_fraction: value.ann_explore_fraction,
            min_not_pruned_pct: value.min_not_pruned_pct,
            max_not_pruned_pct: value.max_not_pruned_pct,
            adjustment_factor_for_partial_d: value.adjustment_factor_for_partial_d,
            unrotate_centroids: value.unrotate_centroids != 0,
            verbose: value.verbose != 0,
            angular: value.angular != 0,
            suppress_warnings: value.suppress_warnings != 0,
            data_already_rotated: value.data_already_rotated != 0,
        }
    }
}

impl From<SkmClusterBalanceStats> for ClusterBalanceStats {
    fn from(value: SkmClusterBalanceStats) -> Self {
        Self {
            mean: value.mean,
            geometric_mean: value.geometric_mean,
            stdev: value.stdev,
            cv: value.cv,
            min: value.min,
            max: value.max,
        }
    }
}

fn to_c_config(config: &HierarchicalSuperKMeansConfig) -> Result<SkmHierConfig> {
    Ok(SkmHierConfig {
        iters: to_u32(config.iters, "config.iters")?,
        sampling_fraction: config.sampling_fraction,
        max_points_per_cluster: to_u32(
            config.max_points_per_cluster,
            "config.max_points_per_cluster",
        )?,
        n_threads: to_u32(config.n_threads, "config.n_threads")?,
        seed: config.seed,
        use_blas_only: config.use_blas_only as u8,
        tol: config.tol,
        recall_tol: config.recall_tol,
        early_termination: config.early_termination as u8,
        sample_queries: config.sample_queries as u8,
        objective_k: config.objective_k,
        ann_explore_fraction: config.ann_explore_fraction,
        min_not_pruned_pct: config.min_not_pruned_pct,
        max_not_pruned_pct: config.max_not_pruned_pct,
        adjustment_factor_for_partial_d: config.adjustment_factor_for_partial_d,
        unrotate_centroids: config.unrotate_centroids as u8,
        verbose: config.verbose as u8,
        angular: config.angular as u8,
        suppress_warnings: config.suppress_warnings as u8,
        data_already_rotated: config.data_already_rotated as u8,
        iters_mesoclustering: to_u32(config.iters_mesoclustering, "config.iters_mesoclustering")?,
        iters_fineclustering: to_u32(config.iters_fineclustering, "config.iters_fineclustering")?,
        iters_refinement: to_u32(config.iters_refinement, "config.iters_refinement")?,
    })
}

fn to_u32(value: usize, name: &str) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| SuperKMeansError::InvalidArgument(format!("{name} must fit into uint32_t")))
}

fn error_from_owned_c_string(error: *mut c_char) -> SuperKMeansError {
    if error.is_null() {
        return SuperKMeansError::Runtime("unknown C++ exception".to_string());
    }
    let message = unsafe { CStr::from_ptr(error) }
        .to_string_lossy()
        .into_owned();
    unsafe { skm_string_free(error) };
    SuperKMeansError::Runtime(message)
}

fn copy_owned_buffer<T: Copy>(ptr: *mut T, len: usize) -> Result<Vec<T>> {
    if ptr.is_null() && len > 0 {
        return Err(SuperKMeansError::Runtime(
            "C++ bridge returned a null buffer".to_string(),
        ));
    }
    let values = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
    };
    unsafe { skm_buffer_free(ptr.cast::<c_void>()) };
    Ok(values)
}
