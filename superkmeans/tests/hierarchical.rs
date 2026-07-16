use superkmeans::{HierarchicalSuperKMeans, HierarchicalSuperKMeansConfig};

#[test]
fn config_synchronizes_with_base() {
    let mut config = HierarchicalSuperKMeansConfig {
        data_already_rotated: true,
        unrotate_centroids: true,
        iters_mesoclustering: 7,
        iters_fineclustering: 9,
        iters_refinement: 3,
        seed: 123,
        sampling_fraction: 0.5,
        ..HierarchicalSuperKMeansConfig::default()
    };
    config.suppress_warnings = true;
    let kmeans = HierarchicalSuperKMeans::with_config(256, 128, config).unwrap();
    assert!(!kmeans.hierarchical_config.unrotate_centroids);
    assert_eq!(kmeans.hierarchical_config.iters_mesoclustering, 7);
    assert_eq!(kmeans.hierarchical_config.iters_fineclustering, 9);
    assert_eq!(kmeans.hierarchical_config.iters_refinement, 3);
    assert_eq!(kmeans.hierarchical_config.seed, 123);
    assert_eq!(kmeans.hierarchical_config.sampling_fraction, 0.5);
}

#[test]
fn basic_training_small_dataset() {
    let n = 2_000;
    let d = 32;
    let n_clusters = 32;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 42);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 3,
        iters_fineclustering: 3,
        iters_refinement: 1,
        early_termination: false,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    assert!(!kmeans.is_trained());
    let centroids = kmeans.train(&data, n).unwrap();
    assert!(kmeans.is_trained());
    assert_eq!(centroids.len(), n_clusters * d);
    let assignments = kmeans.assign(&data, &centroids, n, n_clusters).unwrap();
    assert_eq!(assignments.len(), n);
    assert!(assignments
        .iter()
        .all(|&assignment| assignment < n_clusters as u32));
}

#[test]
fn invalid_inputs_return_errors() {
    assert!(HierarchicalSuperKMeans::new(0, 64).is_err());
    assert!(HierarchicalSuperKMeans::new(16, 0).is_err());
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 0,
        ..HierarchicalSuperKMeansConfig::default()
    };
    assert!(HierarchicalSuperKMeans::with_config(16, 64, config).is_err());
    let config = HierarchicalSuperKMeansConfig {
        sampling_fraction: 1.5,
        ..HierarchicalSuperKMeansConfig::default()
    };
    assert!(HierarchicalSuperKMeans::with_config(16, 64, config).is_err());

    let n = 128;
    let d = 16;
    let data = make_blobs(n, d, 16, 1.0, 10.0, 9);
    let mut kmeans = HierarchicalSuperKMeans::new(16, d).unwrap();
    assert!(kmeans.train(&data, n).is_ok());
    assert!(kmeans.train(&data, n).is_err());
}

#[test]
fn refinement_pruning_path_runs() {
    let n = 700;
    let d = 128;
    let n_clusters = 300;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 7);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 1,
        iters_fineclustering: 1,
        iters_refinement: 1,
        early_termination: false,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    let centroids = kmeans.train(&data, n).unwrap();
    assert_eq!(centroids.len(), n_clusters * d);
    assert_eq!(
        kmeans
            .hierarchical_iteration_stats
            .refinement_iteration_stats
            .len(),
        1
    );
}

#[test]
fn train_with_queries_is_accepted_and_ignored() {
    let n = 512;
    let d = 32;
    let n_clusters = 32;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 10);
    let queries = make_blobs(8, d, n_clusters, 1.0, 10.0, 11);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 2,
        iters_fineclustering: 2,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    let centroids = kmeans
        .train_with_queries(&data, n, Some(&queries), 8)
        .unwrap();
    assert_eq!(centroids.len(), n_clusters * d);
}

#[test]
fn iteration_stats_have_expected_counts_without_early_termination() {
    let n = 1_200;
    let d = 64;
    let n_clusters = 64;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 12);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 3,
        iters_fineclustering: 2,
        iters_refinement: 2,
        early_termination: false,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    kmeans.train(&data, n).unwrap();
    let n_mesoclusters = HierarchicalSuperKMeans::get_n_mesoclusters(n_clusters);
    assert_eq!(
        kmeans
            .hierarchical_iteration_stats
            .mesoclustering_iteration_stats
            .len(),
        3
    );
    assert_eq!(
        kmeans
            .hierarchical_iteration_stats
            .fineclustering_iteration_stats
            .len(),
        n_mesoclusters * 2
    );
    assert_eq!(
        kmeans
            .hierarchical_iteration_stats
            .refinement_iteration_stats
            .len(),
        2
    );
}

#[test]
fn angular_mode_normalizes_centroids() {
    let n = 900;
    let d = 64;
    let n_clusters = 64;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 13);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 2,
        iters_fineclustering: 2,
        iters_refinement: 1,
        angular: true,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    let centroids = kmeans.train(&data, n).unwrap();
    for centroid in centroids.chunks_exact(d) {
        let norm = centroid
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "norm={norm}");
    }
}

#[test]
fn assign_training_points_matches_brute_force_closely() {
    let n = 900;
    let d = 128;
    let n_clusters = 300;
    let data = make_blobs(n, d, n_clusters, 1.0, 10.0, 14);
    let config = HierarchicalSuperKMeansConfig {
        iters_mesoclustering: 1,
        iters_fineclustering: 1,
        iters_refinement: 1,
        early_termination: false,
        suppress_warnings: true,
        ..HierarchicalSuperKMeansConfig::default()
    };
    let mut kmeans = HierarchicalSuperKMeans::with_config(n_clusters, d, config).unwrap();
    let centroids = kmeans.train(&data, n).unwrap();
    let fast = kmeans
        .assign_training_points(&data, &centroids, n, n_clusters)
        .unwrap();
    let brute = kmeans.assign(&data, &centroids, n, n_clusters).unwrap();
    let matches = fast
        .iter()
        .zip(brute.iter())
        .filter(|(left, right)| left == right)
        .count();
    let ratio = matches as f32 / n as f32;
    assert!(ratio >= 0.98, "ratio={ratio}");
}

fn make_blobs(
    n_samples: usize,
    n_features: usize,
    n_centers: usize,
    cluster_std: f32,
    center_spread: f32,
    random_state: u32,
) -> Vec<f32> {
    let mut rng = TestRng::new(random_state);
    let mut centers = vec![0.0; n_centers * n_features];
    for value in &mut centers {
        *value = rng.normal(0.0, center_spread);
    }

    let mut data = vec![0.0; n_samples * n_features];
    for i in 0..n_samples {
        let center = rng.usize(n_centers) * n_features;
        for j in 0..n_features {
            data[i * n_features + j] = centers[center + j] + rng.normal(0.0, cluster_std);
        }
    }
    data
}

struct TestRng {
    state: u64,
}

impl TestRng {
    fn new(seed: u32) -> Self {
        Self {
            state: seed as u64 ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn f32(&mut self) -> f32 {
        self.next_u32() as f32 / (u32::MAX as f32 + 1.0)
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }

    fn normal(&mut self, mean: f32, stddev: f32) -> f32 {
        let u1 = 1.0 - self.f32();
        let u2 = self.f32();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = std::f32::consts::TAU * u2;
        mean + stddev * radius * theta.cos()
    }
}
