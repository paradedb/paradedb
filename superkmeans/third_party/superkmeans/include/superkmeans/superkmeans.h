#pragma once

#include <Eigen/Dense>
#include <algorithm>
#include <iomanip>
#include <omp.h>
#include <random>

#include "superkmeans/common.h"
#include "superkmeans/distance_computers/base_computers.h"
#include "superkmeans/distance_computers/batch_computers.h"
#include "superkmeans/pdx/pdxearch.h"
#include "superkmeans/pdx/utils.h"
#include "superkmeans/profiler.h"

namespace skmeans {

/**
 * @brief Configuration parameters for SuperKMeans clustering.
 * Can be passed to the SuperKMeans constructor.
 *
 */
struct SuperKMeansConfig {
    // Training parameters
    uint32_t iters = 10; // Number of k-means iterations
    // We provide 2 ways to define the number of points to sample:
    float sampling_fraction = 0.3f; // Fraction of data to sample (0.0 to 1.0)
    uint32_t max_points_per_cluster =
        256;                    // Maximum number of points per cluster to sample (FAISS style)
    uint32_t n_threads = 0;     // Number of CPU threads (0 = max)
    uint32_t seed = 42;         // Random seed for reproducibility
    bool use_blas_only = false; // Use BLAS-only computation for all iterations

    // Convergence parameters
    float tol = 1e-4f;                  // Tolerance for shift-based early termination
    float recall_tol = 0.005f;          // Tolerance for recall-based early termination
    bool early_termination = true;      // Whether to stop early on convergence
    bool sample_queries = false;        // Whether to sample queries from data
    size_t objective_k = 100;           // Number of nearest neighbors for recall computation
    float ann_explore_fraction = 0.01f; // Fraction of centroids to explore (0.0 to 1.0)

    // Sweet range for d' tuning
    float min_not_pruned_pct = 0.03f; // Minimum percentage of vectors not pruned (3% = 97% pruned)
    float max_not_pruned_pct = 0.05f; // Maximum percentage of vectors not pruned (5% = 95% pruned)
    // Adjustment factor for d' tuning when outside the sweet range
    float adjustment_factor_for_partial_d = 0.20f;

    // Output parameters
    bool unrotate_centroids = true; // Whether to unrotate centroids before returning
    bool verbose = false;           // Whether to print progress information
    bool angular = false;           // Whether to use spherical k-means
    bool suppress_warnings = false; // Whether to suppress warnings

    bool data_already_rotated = false; // Whether input data is already rotated (skip rotation)
};

/**
 * @brief Statistics for a single iteration of SuperKMeans clustering.
 */
struct SuperKMeansIterationStats {
    size_t iteration = 0;   // Iteration number (1-indexed)
    float objective = 0.0f; // Total clustering cost (WCSS)
    float shift = 0.0f;     // Average squared centroid shift from previous iteration
    size_t split = 0;       // Number of clusters that were split (empty cluster handling)
    float recall = 0.0f;    // Recall@k value (0.0 to 1.0, only when queries provided)
    // Percentage of vectors not pruned (0.0 to 1.0, -1.0 if not applicable)
    float not_pruned_pct = -1.0f;
    // Number of dimensions used for partial distance computation (d')
    uint32_t partial_d = 0;
    // Whether this iteration used BLAS-only computation (no PDX pruning)
    bool is_gemm_only = false;
};

/**
 * @brief Statistics about cluster size balance.
 */
struct ClusterBalanceStats {
    float mean = 0.0f;
    float geometric_mean = 0.0f;
    float stdev = 0.0f;
    float cv = 0.0f;
    size_t min = 0;
    size_t max = 0;

    std::string to_json() const {
        std::ostringstream oss;
        oss << "{\"mean\":" << mean << ",\"geometric_mean\":" << geometric_mean
            << ",\"stdev\":" << stdev << ",\"cv\":" << cv << ",\"min\":" << min
            << ",\"max\":" << max << "}";
        return oss.str();
    }

    void print() const {
        std::cout << "Cluster size stats: "
                  << "mean=" << mean << ", gmean=" << geometric_mean << ", std=" << stdev
                  << ", CV=" << cv << ", min=" << min << ", max=" << max << std::endl;
    }
};

template <Quantization q = Quantization::f32, DistanceFunction alpha = DistanceFunction::l2>
class SuperKMeans {
  public:
    virtual ~SuperKMeans() = default;

  protected:
    using centroid_value_t = skmeans_centroid_value_t<q>;
    using vector_value_t = skmeans_value_t<q>;
    using pruner_t = ADSamplingPruner<q>;
    using layout_t = PDXLayout<q, alpha>;
    using distance_t = skmeans_distance_t<q>;
    using MatrixR = Eigen::Matrix<float, Eigen::Dynamic, Eigen::Dynamic, Eigen::RowMajor>;
    using VectorR = Eigen::VectorXf;
    using batch_computer = BatchComputer<alpha, q>;

  public:
    /**
     * @brief Constructor with custom configuration
     *
     * @param n_clusters Number of clusters to create
     * @param dimensionality Number of dimensions in the data
     * @param config Configuration parameters (see SuperKMeansConfig)
     */
    SuperKMeans(size_t n_clusters, size_t dimensionality, const SuperKMeansConfig& config)
        : d(dimensionality), n_clusters(n_clusters), config(config) {
        SKMEANS_ENSURE_POSITIVE(n_clusters);
        SKMEANS_ENSURE_POSITIVE(dimensionality);
        SKMEANS_ENSURE_POSITIVE(config.iters);
        SKMEANS_ENSURE_POSITIVE(config.sampling_fraction);
        if (config.sampling_fraction > 1.0) {
            throw std::invalid_argument("sampling_fraction must be <= 1.0");
        }
        n_threads = (config.n_threads == 0) ? omp_get_max_threads() : config.n_threads;
        g_n_threads = n_threads;
        pruner = std::make_unique<pruner_t>(dimensionality, PRUNER_INITIAL_THRESHOLD, config.seed);

        // If data is already rotated, we must not unrotate output centroids
        if (this->config.data_already_rotated) {
            this->config.unrotate_centroids = false;
        }
    }

    /**
     * @brief Default constructor
     *
     * @param n_clusters Number of clusters to create
     * @param dimensionality Number of dimensions in the data
     */
    SuperKMeans(size_t n_clusters, size_t dimensionality)
        : SuperKMeans(n_clusters, dimensionality, SuperKMeansConfig{}) {}

    /**
     * @brief Run k-means clustering to determine centroids
     *
     * @param data Pointer to the data matrix (row-major, n × d)
     * @param n Number of points (rows) in the data matrix
     * @param queries Optional pointer to query vectors for recall computation
     * @param n_queries Number of query vectors (ignored if queries is nullptr and sample_queries is
     * false)
     *
     * @return std::vector<skmeans_centroid_value_t<q>> Trained centroids
     */
    std::vector<skmeans_centroid_value_t<q>> Train(
        const vector_value_t* SKM_RESTRICT data,
        const size_t n,
        const vector_value_t* SKM_RESTRICT queries = nullptr,
        const size_t n_queries = 0
    ) {
        SKMEANS_ENSURE_POSITIVE(n);
        if (trained) {
            throw std::runtime_error("The clustering has already been trained");
        }
        iteration_stats.clear();
        if (n < n_clusters) {
            throw std::runtime_error(
                "The number of points should be at least as large as the number of clusters"
            );
        }
        if (n_queries > 0 && queries == nullptr && !config.sample_queries) {
            throw std::invalid_argument(
                "Queries must be provided if n_queries > 0 and sample_queries is false"
            );
        }
        const vector_value_t* SKM_RESTRICT data_p = data;
        n_samples = GetNVectorsToSample(n, n_clusters);
        if (n_samples < n_clusters) {
            throw std::runtime_error(
                "Not enough samples to train. Try increasing the sampling_fraction or "
                "max_points_per_cluster"
            );
        }
        {
            SKM_PROFILE_SCOPE("allocator");
            centroids.reset(new centroid_value_t[n_clusters * d]);
            horizontal_centroids.reset(new centroid_value_t[n_clusters * d]);
            prev_centroids.reset(new centroid_value_t[n_clusters * d]);
            cluster_sizes.reset(new uint32_t[n_clusters]);
            assignments.reset(new uint32_t[n]);
            distances.reset(new distance_t[n]);
            data_norms.reset(new vector_value_t[n_samples]);
            centroid_norms.reset(new vector_value_t[n_clusters]);
        }
        std::vector<vector_value_t> centroids_partial_norms;
        centroids_partial_norms.reserve(n_clusters);
        std::vector<size_t> not_pruned_counts;
        not_pruned_counts.reserve(n_samples);
        std::vector<distance_t> tmp_distances_buf;
        tmp_distances_buf.reserve(X_BATCH_SIZE * Y_BATCH_SIZE);
        vertical_d = PDXLayout<q, alpha>::GetDimensionSplit(d).vertical_d;
        partial_horizontal_centroids.reset(new centroid_value_t[n_clusters * vertical_d]);

        // Set partial_d (d') dynamically as half of vertical_d (around 12% of d)
        partial_d = std::max<uint32_t>(MIN_PARTIAL_D, vertical_d / 2);
        if (partial_d > vertical_d) {
            partial_d = vertical_d;
        }
        if (config.verbose) {
            std::cout << "Front dimensions (d') = " << partial_d << std::endl;
            std::cout << "Trailing dimensions (d'') = " << d - vertical_d << std::endl;
        }

        auto centroids_pdx_wrapper =
            GenerateCentroids(data_p, n_samples, n_clusters, !config.data_already_rotated);
        if (config.verbose) {
            std::cout << "Sampling data..." << std::endl;
        }

        std::vector<vector_value_t> data_samples_buffer;
        data_samples_buffer.reserve(n_samples * d);
        auto data_to_cluster = SampleAndRotateVectors(
            data_p, data_samples_buffer.data(), n, n_samples, !config.data_already_rotated
        );

        RotateOrCopy(
            horizontal_centroids.get(),
            prev_centroids.get(),
            n_clusters,
            !config.data_already_rotated
        );

        GetL2NormsRowMajor(data_to_cluster, n_samples, data_norms.get());
        GetL2NormsRowMajor(prev_centroids.get(), n_clusters, centroid_norms.get());

        std::vector<vector_value_t> rotated_queries;
        if (n_queries) {
            centroids_to_explore =
                std::max<size_t>(static_cast<size_t>(n_clusters * config.ann_explore_fraction), 1);
            if (config.verbose) {
                std::cout << "Centroids to explore: " << centroids_to_explore << " ("
                          << config.ann_explore_fraction * 100.0f << "% of " << n_clusters << ")"
                          << std::endl;
            }
            {
                SKM_PROFILE_SCOPE("allocator");
                gt_assignments.reset(new uint32_t[n_queries * config.objective_k]);
                gt_distances.reset(new distance_t[n_queries * config.objective_k]);
                tmp_distances_buffer.reset(new distance_t[X_BATCH_SIZE * Y_BATCH_SIZE]);
                promising_centroids.reset(new uint32_t[n_queries * centroids_to_explore]);
                recall_distances.reset(new distance_t[n_queries * centroids_to_explore]);
                query_norms.reset(new distance_t[n_queries]);
            }
            rotated_queries.reserve(n_queries * d);
            if (config.sample_queries) {
                std::cout << "Sampling queries from data..." << std::endl;
                SampleAndRotateVectors(
                    data_to_cluster, rotated_queries.data(), n_samples, n_queries, false
                );
            } else {
                RotateOrCopy(
                    queries, rotated_queries.data(), n_queries, !config.data_already_rotated
                );
            }
            GetL2NormsRowMajor(rotated_queries.data(), n_queries, query_norms.get());
            GetGTAssignmentsAndDistances(data_to_cluster, rotated_queries.data(), n_queries);
        }

        bool always_gemm_only = d < DIMENSION_THRESHOLD_FOR_PRUNING || config.use_blas_only ||
                                n_clusters <= N_CLUSTERS_THRESHOLD_FOR_PRUNING;
        bool partial_norms_computed = false;
        float best_recall = 0.0f;
        size_t iters_without_improvement = 0;

        for (size_t iter_idx = 0; iter_idx < config.iters; ++iter_idx) {
            bool use_gemm_only = (iter_idx == 0) || always_gemm_only;
            if (!use_gemm_only && !partial_norms_computed) {
                GetPartialL2NormsRowMajor(data_to_cluster, n_samples, data_norms.get(), partial_d);
                partial_norms_computed = true;
            }
            if (use_gemm_only) {
                RunIteration<true>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    centroids_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    rotated_queries.data(),
                    n_queries,
                    n_samples,
                    n_clusters,
                    iter_idx,
                    iter_idx == 0,
                    iteration_stats
                );
            } else {
                RunIteration<false>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    centroids_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    rotated_queries.data(),
                    n_queries,
                    n_samples,
                    n_clusters,
                    iter_idx,
                    false,
                    iteration_stats
                );
            }
            if (config.early_termination &&
                ShouldStopEarly(n_queries > 0, best_recall, iters_without_improvement, iter_idx)) {
                break;
            }
        }

        trained = true;

        auto output_centroids = GetOutputCentroids(config.unrotate_centroids);
        if (config.verbose) {
            Profiler::Get().PrintHierarchical();
        }
        return output_centroids;
    }

    /**
     * @brief Assign vectors to their nearest centroid using brute force search.
     *
     * The vectors and centroids are assumed to be in the same domain
     * (no rotation/transformation needed).
     *
     * @param vectors The data matrix (row-major, n_vectors x d)
     * @param centroids The centroids matrix (row-major, n_centroids x d)
     * @param n_vectors Number of vectors
     * @param n_centroids Number of centroids
     * @return std::vector<uint32_t> Assignment for each vector (index of nearest centroid)
     */
    [[nodiscard]] std::vector<uint32_t> Assign(
        const vector_value_t* SKM_RESTRICT vectors,
        const vector_value_t* SKM_RESTRICT centroids,
        const size_t n_vectors,
        const size_t n_centroids
    ) {
        SKM_PROFILE_SCOPE("assign");
        std::vector<uint32_t> result_assignments(n_vectors);
        std::unique_ptr<distance_t[]> tmp_distances_buf(new distance_t[X_BATCH_SIZE * Y_BATCH_SIZE]
        );
        std::vector<vector_value_t> vector_norms(n_vectors);
        std::vector<vector_value_t> centroid_norms_local(n_centroids);
        std::vector<distance_t> result_distances(n_vectors);

        Eigen::Map<const MatrixR> vectors_mat(vectors, n_vectors, d);
        Eigen::Map<VectorR> v_norms(vector_norms.data(), n_vectors);
        v_norms.noalias() = vectors_mat.rowwise().squaredNorm();

        Eigen::Map<const MatrixR> centroids_mat(centroids, n_centroids, d);
        Eigen::Map<VectorR> c_norms(centroid_norms_local.data(), n_centroids);
        c_norms.noalias() = centroids_mat.rowwise().squaredNorm();

        batch_computer::FindNearestNeighbor(
            vectors,
            centroids,
            n_vectors,
            n_centroids,
            d,
            vector_norms.data(),
            centroid_norms_local.data(),
            result_assignments.data(),
            result_distances.data(),
            tmp_distances_buf.get()
        );

        return result_assignments;
    }

    /**
     * @brief Fast assignment using GEMM+PRUNING with trained state.
     *
     * Requires that the vectors passed here are the same as those used in .Train().
     * Leverages the assignments from training for a faster
     * assignment than brute force Assign().
     *
     * @param vectors The training data matrix (row-major, n_vectors x d)
     * @param centroids The centroids matrix (row-major, n_centroids x d)
     * @param n_vectors Number of vectors
     * @param n_centroids Number of centroids
     * @return std::vector<uint32_t> Assignment for each vector (index of nearest centroid)
     */
    [[nodiscard]] std::vector<uint32_t> AssignTrainingPoints(
        const vector_value_t* SKM_RESTRICT vectors,
        const vector_value_t* SKM_RESTRICT centroids,
        const size_t n_vectors,
        const size_t n_centroids
    ) {
        SKM_PROFILE_SCOPE("assign_training_points");
        if (!trained) {
            throw std::runtime_error("AssignTrainingPoints requires SuperKMeans to be trained first"
            );
        }

        if (config.use_blas_only || d < DIMENSION_THRESHOLD_FOR_PRUNING ||
            n_clusters <= N_CLUSTERS_THRESHOLD_FOR_PRUNING) {
            if (!config.suppress_warnings) {
                std::cout << "WARNING: AssignTrainingPoints cannot use pruning, falling back to "
                             "brute force Assign"
                          << std::endl;
            }
            return Assign(vectors, centroids, n_vectors, n_centroids);
        }
        if (config.verbose) {
            Profiler::Get().Reset();
        }

        std::vector<uint32_t> result_assignments(n_vectors);
        std::vector<distance_t> tmp_distances_buf(X_BATCH_SIZE * Y_BATCH_SIZE);

        partial_d = std::max<uint32_t>(MIN_PARTIAL_D, vertical_d / 2);

        std::vector<size_t> not_pruned_counts;
        not_pruned_counts.reserve(n_vectors);
        std::fill(not_pruned_counts.data(), not_pruned_counts.data() + n_vectors, 0);
        std::vector<vector_value_t> data_buffer;
        const vector_value_t* data_p;
        if (config.data_already_rotated) {
            // Data is already rotated: use original pointer directly (avoid redundant memcpy)
            data_p = vectors;
        } else {
            data_buffer.reserve(n_vectors * d);
            RotateOrCopy(vectors, data_buffer.data(), n_vectors, true);
            data_p = data_buffer.data();
        }
        GetPartialL2NormsRowMajor(
            horizontal_centroids.get(), n_centroids, centroid_norms.get(), partial_d
        );

        // Consolidate was called at the end of RunIteration<true>, so we don't need to call it here
        // All the centroid-related pointers are updated with the final centroids
        auto pdx_centroids = PDXLayout<q, alpha>(
            this->centroids.get(), *pruner, n_clusters, d, partial_horizontal_centroids.get()
        );

        // If nothing was sampled, then we just go ahead with GEMM+PRUNING
        if (config.sampling_fraction == 1.0f) {
            // Recompute data norms defensively (data_p is independently rotated)
            GetPartialL2NormsRowMajor(data_p, n_vectors, data_norms.get(), partial_d);
            batch_computer::FindNearestNeighborWithPruning(
                data_p,
                horizontal_centroids.get(),
                n_vectors,
                n_clusters,
                d,
                data_norms.get(),
                centroid_norms.get(),
                assignments.get(),
                distances.get(),
                tmp_distances_buf.data(),
                pdx_centroids,
                partial_d,
                not_pruned_counts.data()
            );
            memcpy(result_assignments.data(), assignments.get(), n_vectors * sizeof(uint32_t));
            return result_assignments;
        } else if (config.sampling_fraction > 0.8f) {
            // Dereference the current assignments from the sampled_indices
            size_t cur_vector_idx = 0;
            for (; cur_vector_idx < n_samples; ++cur_vector_idx) {
                result_assignments[sampled_indices[cur_vector_idx]] = assignments[cur_vector_idx];
            }
            // Seed remaining vectors with a cluster drawn proportionally to cluster size
            std::mt19937 rng(config.seed + 1);
            std::discrete_distribution<uint32_t> cluster_dist(
                cluster_sizes.get(), cluster_sizes.get() + n_clusters
            );
            for (; cur_vector_idx < n_vectors; ++cur_vector_idx) {
                result_assignments[sampled_indices[cur_vector_idx]] = cluster_dist(rng);
            }

            // data_norms was allocated for n_samples in Train(), reallocate for n_vectors
            data_norms.reset(new vector_value_t[n_vectors]);
            GetPartialL2NormsRowMajor(data_p, n_vectors, data_norms.get(), partial_d);
            batch_computer::FindNearestNeighborWithPruning(
                data_p,
                horizontal_centroids.get(),
                n_vectors,
                n_clusters,
                d,
                data_norms.get(),
                centroid_norms.get(),
                result_assignments.data(),
                distances.get(),
                tmp_distances_buf.data(),
                pdx_centroids,
                partial_d,
                not_pruned_counts.data()
            );
            return result_assignments;
        } else {
            // When sampling_fraction is very low we don't have good initial assignments.
            // We obtain a good initial assignment by clustering the given centroids into
            // sqrt(n_centroids) meso-clusters, then map each vector's meso-assignment
            // back to a representative original centroid for seeding.
            SuperKMeansConfig tmp_config;
            tmp_config.iters = 10;
            tmp_config.sampling_fraction = 1.0f;
            tmp_config.use_blas_only = false;
            tmp_config.verbose = config.verbose;
            tmp_config.suppress_warnings = config.suppress_warnings;
            tmp_config.seed = config.seed;
            tmp_config.angular = config.angular;
            tmp_config.data_already_rotated = config.data_already_rotated;
            auto new_n_centroids = static_cast<size_t>(std::sqrt(n_centroids));
            SuperKMeans tmp_kmeans(new_n_centroids, d, tmp_config);
            auto meso_centroids = tmp_kmeans.Train(centroids, n_centroids);
            auto meso_assignments =
                tmp_kmeans.Assign(vectors, meso_centroids.data(), n_vectors, new_n_centroids);

            // Map each meso-centroid to a single representative original centroid
            auto centroids_to_meso =
                tmp_kmeans.Assign(centroids, meso_centroids.data(), n_centroids, new_n_centroids);
            std::vector<uint32_t> meso_to_original(new_n_centroids, 0);
            for (size_t c = 0; c < n_centroids; ++c) {
                meso_to_original[centroids_to_meso[c]] = static_cast<uint32_t>(c);
            }

            // Seed sampled vectors from training assignments
            size_t cur_vector_idx = 0;
            for (; cur_vector_idx < n_samples; ++cur_vector_idx) {
                result_assignments[sampled_indices[cur_vector_idx]] = assignments[cur_vector_idx];
            }
            // Seed non-sampled vectors: map their meso-assignment to an original centroid
            for (; cur_vector_idx < n_vectors; ++cur_vector_idx) {
                size_t orig_idx = sampled_indices[cur_vector_idx];
                result_assignments[orig_idx] = meso_to_original[meso_assignments[orig_idx]];
            }

            data_norms.reset(new vector_value_t[n_vectors]);
            GetPartialL2NormsRowMajor(data_p, n_vectors, data_norms.get(), partial_d);

            batch_computer::FindNearestNeighborWithPruning(
                data_p,
                horizontal_centroids.get(),
                n_vectors,
                n_clusters,
                d,
                data_norms.get(),
                centroid_norms.get(),
                result_assignments.data(),
                distances.get(),
                tmp_distances_buf.data(),
                pdx_centroids,
                partial_d,
                not_pruned_counts.data()
            );
            return result_assignments;
        }
    }

    /** @brief Returns the number of clusters. */
    [[nodiscard]] size_t GetNClusters() const noexcept { return n_clusters; }

    /** @brief Returns whether the model has been trained. */
    [[nodiscard]] bool IsTrained() const noexcept { return trained; }

    /** @brief Returns the sampling fraction used during training. */
    [[nodiscard]] float GetSamplingFraction() const noexcept { return config.sampling_fraction; }

    /** @brief Returns a pointer to the distances array. */
    [[nodiscard]] distance_t* GetDistancesPointer() { return distances.get(); }

    /**
     * @brief Calculate cluster balance statistics from assignments
     *
     * @param assignments Array of cluster assignments [n_samples]
     * @param n_samples Number of samples
     * @param n_clusters Number of clusters
     * @return ClusterBalanceStats containing mean, stdev, CV, min, max
     */
    [[nodiscard]] static ClusterBalanceStats GetClustersBalanceStats(
        const uint32_t* assignments,
        size_t n_samples,
        size_t n_clusters
    ) {
        ClusterBalanceStats stats;
        std::vector<size_t> cluster_sizes(n_clusters, 0);
        for (size_t i = 0; i < n_samples; ++i) {
            cluster_sizes[assignments[i]]++;
        }

        auto sum = std::accumulate(cluster_sizes.begin(), cluster_sizes.end(), size_t{0});
        stats.mean = static_cast<float>(sum) / static_cast<float>(cluster_sizes.size());

        // Geometric mean
        float log_sum = 0.0f;
        size_t non_zero_count = 0;
        for (size_t size : cluster_sizes) {
            if (size > 0) {
                log_sum += std::log(static_cast<float>(size));
                non_zero_count++;
            }
        }
        stats.geometric_mean =
            (non_zero_count > 0) ? std::exp(log_sum / static_cast<float>(non_zero_count)) : 0.0f;

        auto sq_sum = std::inner_product(
            cluster_sizes.begin(), cluster_sizes.end(), cluster_sizes.begin(), size_t{0}
        );
        stats.stdev = std::sqrt(
            static_cast<float>(sq_sum) / static_cast<float>(cluster_sizes.size()) -
            stats.mean * stats.mean
        );

        // Coefficient of variation
        stats.cv = stats.stdev / stats.mean;

        auto minmax = std::minmax_element(cluster_sizes.begin(), cluster_sizes.end());
        stats.min = *minmax.first;
        stats.max = *minmax.second;

        return stats;
    }

  protected:
    /**
     * @brief Performs first assignment and centroid update using FULL GEMM.
     *
     * Used for the first iteration where full distance computation via GEMM is used
     * (no pruning). Assigns each data point to its nearest centroid, then updates
     * centroid positions.
     *
     * @param data Data matrix (row-major, n_samples × d)
     * @param rotated_initial_centroids Initial centroids (row-major, n_clusters × d)
     * @param tmp_distances_buf Workspace buffer for distance computations
     * @param n_samples Number of vectors in the data
     * @param n_clusters Number of centroids
     */
    void FirstAssignAndUpdateCentroids(
        const vector_value_t* SKM_RESTRICT data,
        const vector_value_t* SKM_RESTRICT rotated_initial_centroids,
        distance_t* SKM_RESTRICT tmp_distances_buf,
        const size_t n_samples,
        const size_t n_clusters
    ) {
        batch_computer::FindNearestNeighbor(
            data,
            rotated_initial_centroids,
            n_samples,
            n_clusters,
            d,
            data_norms.get(),
            centroid_norms.get(),
            assignments.get(),
            distances.get(),
            tmp_distances_buf
        );
        {
            SKM_PROFILE_SCOPE("fill");
            std::fill(
                horizontal_centroids.get(), horizontal_centroids.get() + (n_clusters * d), 0.0
            );
            std::fill(cluster_sizes.get(), cluster_sizes.get() + n_clusters, 0);
        }
    }

    /**
     * @brief Performs assignment and centroid update using GEMM+PRUNING.
     *
     * Uses GEMM for partial distance computation (first partial_d dimensions),
     * then PRUNING for completing distances for remaining candidates.
     *
     * @param data Data matrix (row-major, n_samples × d)
     * @param centroids Centroids to use for GEMM distance computation (row-major)
     * @param partial_centroid_norms Partial norms of centroids (first partial_d dims)
     * @param tmp_distances_buf Workspace buffer for distance computations
     * @param pdx_centroids PDX-layout centroids for PRUNING
     * @param out_not_pruned_counts Output for pruning statistics
     */
    void AssignAndUpdateCentroids(
        const vector_value_t* SKM_RESTRICT data,
        const vector_value_t* SKM_RESTRICT centroids,
        const vector_value_t* SKM_RESTRICT partial_centroid_norms,
        distance_t* SKM_RESTRICT tmp_distances_buf,
        const layout_t& pdx_centroids,
        size_t* out_not_pruned_counts,
        const size_t n_samples,
        const size_t n_clusters
    ) {
        batch_computer::FindNearestNeighborWithPruning(
            data,
            centroids,
            n_samples,
            n_clusters,
            d,
            data_norms.get(),
            partial_centroid_norms,
            assignments.get(),
            distances.get(),
            tmp_distances_buf,
            pdx_centroids,
            partial_d,
            out_not_pruned_counts
        );
        {
            SKM_PROFILE_SCOPE("fill");
            std::fill(
                horizontal_centroids.get(), horizontal_centroids.get() + (n_clusters * d), 0.0
            );
            std::fill(cluster_sizes.get(), cluster_sizes.get() + n_clusters, 0);
        }
    }

    /**
     * @brief Updates centroids by accumulating assigned vectors.
     *
     * After this call, horizontal_centroids contains the sum of assigned vectors.
     * ConsolidateCentroids() must be called to normalize by cluster sizes.
     *
     * @param data Data matrix (row-major, n_samples × d)
     * @param n_samples
     * @param n_clusters
     */
    void UpdateCentroids(
        const vector_value_t* SKM_RESTRICT data,
        const size_t n_samples,
        const size_t n_clusters
    ) {
        SKM_PROFILE_SCOPE("update_centroids");
#pragma omp parallel if (n_threads > 1) num_threads(n_threads)
        {
            uint32_t nt = n_threads;
            uint32_t rank = omp_get_thread_num();
            // This thread is taking care of centroids c0:c1
            size_t c0 = (n_clusters * rank) / nt;
            size_t c1 = (n_clusters * (rank + 1)) / nt;
            for (size_t i = 0; i < n_samples; i++) {
                uint32_t ci = assignments[i];
                assert(ci < n_clusters);
                if (ci >= c0 && ci < c1) {
                    auto vector_p = data + i * d;
                    cluster_sizes[ci] += 1;
                    UpdateCentroid(vector_p, ci);
                }
            }
        }
    }

    /**
     * @brief Adds a vector to its assigned centroid's accumulator.
     */
    SKM_ALWAYS_INLINE void UpdateCentroid(
        const vector_value_t* SKM_RESTRICT vector,
        const uint32_t cluster_idx
    ) {
        SKM_VECTORIZE_LOOP
        for (size_t i = 0; i < d; ++i) {
            horizontal_centroids[cluster_idx * d + i] += vector[i];
        }
    }

    /**
     * @brief Runs a single K-Means iteration with either GEMM-only or GEMM+PRUNING strategy.
     *
     *
     * @tparam GEMM_ONLY If true, uses full GEMM (FirstAssignAndUpdateCentroids).
     *                   If false, uses GEMM+PRUNING (AssignAndUpdateCentroids with TunePartialD).
     *
     * @param data_to_cluster Training data (rotated, row-major)
     * @param tmp_distances_buf Workspace buffer for distance computations
     * @param centroids_pdx_wrapper PDX-layout centroids (only used when !GEMM_ONLY)
     * @param centroids_partial_norms Partial norms buffer (only used when !GEMM_ONLY)
     * @param not_pruned_counts Pruning statistics buffer (only used when !GEMM_ONLY)
     * @param rotated_queries Query vectors for recall computation (nullptr if n_queries==0)
     * @param n_queries Number of query vectors
     * @param n_samples Number of training samples
     * @param n_clusters Number of clusters
     * @param iter_idx Current iteration index (0-based)
     * @param is_first_iter Whether this is the first iteration (skips centroid swap)
     */
    template <bool GEMM_ONLY>
    void RunIteration(
        const vector_value_t* SKM_RESTRICT data_to_cluster,
        distance_t* SKM_RESTRICT tmp_distances_buf,
        const layout_t& centroids_pdx_wrapper,
        std::vector<vector_value_t>& centroids_partial_norms,
        std::vector<size_t>& not_pruned_counts,
        const vector_value_t* SKM_RESTRICT rotated_queries,
        const size_t n_queries,
        const size_t n_samples,
        const size_t n_clusters,
        size_t& iter_idx,
        const bool is_first_iter,
        std::vector<SuperKMeansIterationStats>& target_stats
    ) {
        if (!is_first_iter) {
            std::swap(horizontal_centroids, prev_centroids);
        }

        if constexpr (GEMM_ONLY) {
            GetL2NormsRowMajor(prev_centroids.get(), n_clusters, centroid_norms.get());
            FirstAssignAndUpdateCentroids(
                data_to_cluster, prev_centroids.get(), tmp_distances_buf, n_samples, n_clusters
            );
        } else {
            GetPartialL2NormsRowMajor(
                prev_centroids.get(), n_clusters, centroids_partial_norms.data(), partial_d
            );
            {
                SKM_PROFILE_SCOPE("fill");
                std::fill(not_pruned_counts.data(), not_pruned_counts.data() + n_samples, 0);
            }
            AssignAndUpdateCentroids(
                data_to_cluster,
                prev_centroids.get(),
                centroids_partial_norms.data(),
                tmp_distances_buf,
                centroids_pdx_wrapper,
                not_pruned_counts.data(),
                n_samples,
                n_clusters
            );
        }

        UpdateCentroids(data_to_cluster, n_samples, n_clusters);

        float avg_not_pruned_pct = -1.0f;
        uint32_t old_partial_d = partial_d;
        if constexpr (!GEMM_ONLY) {
            bool partial_d_changed = false;
            avg_not_pruned_pct =
                TunePartialD(not_pruned_counts.data(), n_samples, n_clusters, partial_d_changed);
            if (partial_d_changed) {
                GetPartialL2NormsRowMajor(data_to_cluster, n_samples, data_norms.get(), partial_d);
            }
        }

        ConsolidateCentroids(n_samples, n_clusters);

        ComputeCost(n_samples);
        ComputeShift(n_clusters);

        if (n_queries) {
            GetL2NormsRowMajor(horizontal_centroids.get(), n_clusters, centroid_norms.get());
            recall = ComputeRecall(rotated_queries, n_queries);
        }

        SuperKMeansIterationStats stats;
        stats.iteration = iter_idx + 1;
        stats.objective = cost;
        stats.shift = shift;
        stats.split = n_split;
        stats.recall = recall;
        stats.is_gemm_only = GEMM_ONLY;
        if constexpr (!GEMM_ONLY) {
            stats.not_pruned_pct = avg_not_pruned_pct;
            stats.partial_d = old_partial_d;
        }
        target_stats.push_back(stats);

        if (config.verbose) {
            std::cout << "Iteration " << iter_idx + 1 << "/" << config.iters
                      << " | Objective: " << cost << " | Objective improvement: "
                      << (iter_idx > 0 ? 1 - (cost / prev_cost) : 0.0f) << " | Shift: " << shift
                      << " | Split: " << n_split << " | Recall: " << recall;
            if constexpr (GEMM_ONLY) {
                std::cout << " [BLAS-only]";
            } else {
                std::cout << " | Not Pruned %: " << avg_not_pruned_pct * 100.0f
                          << " | d': " << old_partial_d << " -> " << partial_d;
            }
            std::cout << std::endl << std::endl;
        }
    }

    /**
     * @brief Handles empty clusters by splitting large clusters.
     * Taken from FAISS implementation:
     * https://github.com/facebookresearch/faiss/blob/main/faiss/Clustering.cpp
     *
     * When a cluster becomes empty (no points assigned), this method splits
     * a large cluster to repopulate it. Selection is probabilistic based on
     * cluster sizes.
     */
    virtual void SplitClusters(const size_t n_samples, const size_t n_clusters) {
        n_split = 0;
        std::mt19937 rng(config.seed);
        auto horizontal_centroids_p = horizontal_centroids.get();
        for (size_t ci = 0; ci < n_clusters; ci++) {
            if (cluster_sizes[ci] == 0) {
                size_t cj;
                for (cj = 0; true; cj = (cj + 1) % n_clusters) {
                    // Probability to pick this cluster for split
                    float p = (cluster_sizes[cj] - 1.0) / (float) (n_samples - n_clusters);
                    float r = std::uniform_real_distribution<float>(0, 1)(rng);
                    if (r < p) {
                        break; // Found our cluster to be split
                    }
                }
                memcpy(
                    (void*) (horizontal_centroids_p + ci * d),
                    (void*) (horizontal_centroids_p + cj * d),
                    sizeof(centroid_value_t) * d
                );

                // Small symmetric perturbation
                for (size_t j = 0; j < d; j++) {
                    if (j % 2 == 0) {
                        horizontal_centroids_p[ci * d + j] *= 1 + CENTROID_PERTURBATION_EPS;
                        horizontal_centroids_p[cj * d + j] *= 1 - CENTROID_PERTURBATION_EPS;
                    } else {
                        horizontal_centroids_p[ci * d + j] *= 1 - CENTROID_PERTURBATION_EPS;
                        horizontal_centroids_p[cj * d + j] *= 1 + CENTROID_PERTURBATION_EPS;
                    }
                }

                // Assume even split of the cluster
                cluster_sizes[ci] = cluster_sizes[cj] / 2;
                cluster_sizes[cj] -= cluster_sizes[ci];
                n_split++;
            }
        }
    }

    /**
     * @brief Finalizes centroid computation after assignment.
     *
     * Divides accumulated sums by cluster sizes to get mean centroids,
     * handles empty clusters via splitting, and converts to PDX layout.
     */
    void ConsolidateCentroids(const size_t n_samples, const size_t n_clusters) {
        SKM_PROFILE_SCOPE("consolidate");
        {
            SKM_PROFILE_SCOPE("consolidate/splitting");
#pragma omp parallel for if (n_threads > 1) num_threads(n_threads)
            for (size_t i = 0; i < n_clusters; ++i) {
                auto horizontal_centroids_p = horizontal_centroids.get() + i * d;
                if (cluster_sizes[i] == 0) {
                    continue;
                }
                float mult_factor = 1.0 / cluster_sizes[i];
                SKM_VECTORIZE_LOOP
                for (size_t j = 0; j < d; ++j) {
                    horizontal_centroids_p[j] *= mult_factor;
                }
            }
            SplitClusters(n_samples, n_clusters);
        }
        {
            SKM_PROFILE_SCOPE("consolidate/normalize");
            if (config.angular) {
                PostprocessCentroids(n_clusters);
            }
        }
        {
            SKM_PROFILE_SCOPE("consolidate/pdxify");
            //! This updates the object within the pdx_layout wrapper
            PDXLayout<q, alpha>::template PDXify<false>(
                horizontal_centroids.get(), centroids.get(), n_clusters, d
            );
            CentroidsToAuxiliaryHorizontal(n_clusters);
        }
    }

    /**
     * @brief Computes Within-Cluster Sum of Squares (WCSS).
     */
    void ComputeCost(const size_t n_samples) {
        SKM_PROFILE_SCOPE("compute_cost");
        prev_cost = cost;
        cost = 0.0f;
        SKM_VECTORIZE_LOOP
        for (size_t i = 0; i < n_samples; ++i) {
            cost += distances[i];
        }
    }

    /**
     * @brief Computes the squared centroid shift from previous iteration.
     *
     * Used for convergence detection - small shift indicates centroids have stabilized.
     */
    void ComputeShift(const size_t n_clusters) {
        SKM_PROFILE_SCOPE("shift");
        Eigen::Map<const MatrixR> new_mat(horizontal_centroids.get(), n_clusters, d);
        Eigen::Map<const MatrixR> prev_mat(prev_centroids.get(), n_clusters, d);
        float total_shift = 0.0f;
#pragma omp parallel for reduction(+ : total_shift) if (n_threads > 1) num_threads(n_threads)
        for (size_t i = 0; i < n_clusters; ++i) {
            total_shift += (new_mat.row(i) - prev_mat.row(i)).squaredNorm();
        }
        shift = total_shift;
    }

    /**
     * @brief Computes ground truth assignments for recall calculation.
     *
     * Finds the top-k nearest data points for each query using exact search.
     * These assignments are used as ground truth for evaluating centroid quality.
     *
     * @param data Data matrix (sampled data points)
     * @param queries Query vectors
     * @param n_queries Number of query vectors
     */
    void GetGTAssignmentsAndDistances(
        const vector_value_t* SKM_RESTRICT data,
        const vector_value_t* SKM_RESTRICT queries,
        const size_t n_queries
    ) {
        SKM_PROFILE_SCOPE("gt_assignments");
        std::vector<distance_t> gt_query_norms(n_queries);
        GetL2NormsRowMajor(queries, n_queries, gt_query_norms.data());
        batch_computer::FindKNearestNeighbors(
            queries,
            data,
            n_queries,
            n_samples,
            d,
            gt_query_norms.data(),
            data_norms.get(),
            config.objective_k,
            gt_assignments.get(),
            gt_distances.get(),
            tmp_distances_buffer.get()
        );
    }

    /**
     * @brief Computes recall@k for current centroids.
     *
     * For each query, checks how many of its ground truth nearest neighbors
     * would be found when searching only the top centroids. Higher recall
     * indicates better centroid quality for ANN indexing.
     *
     * @param queries Query vectors
     * @param n_queries Number of query vectors
     * @return Recall value (0.0 to 1.0)
     */
    float ComputeRecall(const vector_value_t* SKM_RESTRICT queries, const size_t n_queries) {
        SKM_PROFILE_SCOPE("recall");
        batch_computer::FindKNearestNeighbors(
            queries,
            horizontal_centroids.get(),
            n_queries,
            n_clusters,
            d,
            query_norms.get(),
            centroid_norms.get(),
            centroids_to_explore,
            promising_centroids.get(),
            recall_distances.get(),
            tmp_distances_buffer.get()
        );
        // For each query, compute recall@objective_k: how many of the GT clusters are found in the
        // top-%centroids_to_explore assignments
        // Recall per query = (# matched GT assignments in top-%centroids_to_explore) / objective_k
        // Final recall = average over all queries
        float sum_recall = 0.0f;
        for (size_t i = 0; i < n_queries; ++i) {
            size_t found_in_query = 0;
            // For each GT assignment for query q
            for (size_t j = 0; j < config.objective_k; ++j) {
                uint32_t gt = gt_assignments[i * config.objective_k + j]; // gt is a vector index
                // Check if this GT assignment is present in the top-%centroids_to_explore
                // assignments for this query
                bool found = false;
                for (size_t t = 0; t < centroids_to_explore; ++t) {
                    // If a centroid is the same as the GT centroid assignment, then we have a match
                    if (promising_centroids[i * centroids_to_explore + t] == assignments[gt]) {
                        found = true;
                        break;
                    }
                }
                if (found) {
                    ++found_in_query;
                }
            }
            sum_recall +=
                static_cast<float>(found_in_query) / static_cast<float>(config.objective_k);
        }
        return sum_recall / static_cast<float>(n_queries);
    }

    /**
     * @brief Generates initial centroids from the data.
     *
     * Forgy sampling: Randomly samples n_clusters vectors as initial centroids,
     * rotates them, PDXifies them, and wraps them in a PDXLayout wrapper.
     *
     * @param data Data matrix
     * @param n_clusters Number of centroids to generate
     * @param rotate Wheter to rotate the sampled centroids
     * @return PDXLayout wrapper for the centroids
     */
    PDXLayout<q, alpha> GenerateCentroids(
        const vector_value_t* SKM_RESTRICT data,
        const size_t n_points,
        const size_t n_clusters,
        const bool rotate = true
    ) {
        {
            SKM_PROFILE_SCOPE("generating_centroids");
            auto tmp_centroids_p = horizontal_centroids.get();

            std::mt19937 rng(config.seed);
            std::vector<size_t> indices(n_points);
            for (size_t i = 0; i < n_points; ++i) {
                indices[i] = i;
            }
            std::shuffle(indices.begin(), indices.end(), rng);
            for (size_t i = 0; i < n_clusters; i += 1) {
                memcpy(
                    (void*) tmp_centroids_p,
                    (void*) (data + (indices[i] * d)),
                    sizeof(centroid_value_t) * d
                );
                tmp_centroids_p += d;
            }
        }
        // We populate the centroids buffer with the centroids in the PDX layout
        std::vector<centroid_value_t> rotated_centroids(n_clusters * d);
        RotateOrCopy(horizontal_centroids.get(), rotated_centroids.data(), n_clusters, rotate);
        {
            SKM_PROFILE_SCOPE("consolidate/pdxify");
            PDXLayout<q, alpha>::template PDXify<false>(
                rotated_centroids.data(), centroids.get(), n_clusters, d
            );
        }
        //! We wrap centroids and partial_horizontal_centroids in the PDXLayout wrapper
        //! Any updates to these objects is reflected in the PDXLayout
        //! partial_horizontal_centroids are not filled until ConsolidateCentroids is called()
        // after the first iteration
        auto pdx_centroids = PDXLayout<q, alpha>(
            centroids.get(), *pruner, n_clusters, d, partial_horizontal_centroids.get()
        );
        return pdx_centroids;
    }

    /**
     * @brief Computes partial L2 squared norms (first partial_d dimensions).
     */
    void GetPartialL2NormsRowMajor(
        const vector_value_t* SKM_RESTRICT data,
        const size_t n,
        vector_value_t* SKM_RESTRICT out_norm,
        const size_t partial_d
    ) {
        SKM_PROFILE_SCOPE("norms_calc");
        Eigen::Map<const MatrixR> e_data(data, n, d);
        Eigen::Map<VectorR> e_norms(out_norm, n);
        e_norms.noalias() = e_data.leftCols(partial_d).rowwise().squaredNorm();
    }

    /**
     * @brief Computes full L2 squared norms for each vector.
     */
    void GetL2NormsRowMajor(
        const vector_value_t* SKM_RESTRICT data,
        const size_t n,
        vector_value_t* SKM_RESTRICT out_norm
    ) {
        SKM_PROFILE_SCOPE("norms_calc");
        Eigen::Map<const MatrixR> e_data(data, n, d);
        Eigen::Map<VectorR> e_norms(out_norm, n);
        e_norms.noalias() = e_data.rowwise().squaredNorm();
    }

    /**
     * @brief Rotates or copies vectors based on rotate parameter.
     *
     * If rotate is false, performs a simple memcpy. Otherwise, applies rotation.
     *
     * @param in Input buffer (potentially unrotated)
     * @param out Output buffer (rotated or copied)
     * @param n_vectors Number of vectors to process
     * @param rotate Whether to rotate (true) or just copy (false)
     */
    void RotateOrCopy(
        const centroid_value_t* SKM_RESTRICT in,
        centroid_value_t* SKM_RESTRICT out,
        const size_t n_vectors,
        const bool rotate
    ) {
        SKM_PROFILE_SCOPE("rotator");
        if (rotate) { // NOLINT(bugprone-branch-clone)
            pruner->Rotate(in, out, n_vectors);
        } else {
            memcpy(
                static_cast<void*>(out),
                static_cast<const void*>(in),
                sizeof(centroid_value_t) * n_vectors * d
            );
        }
    }

    /**
     * @brief Copies the first vertical_d dimensions of centroids for efficient PRUNING
     * TODO(@lkuffo, high): We can avoid this by using the full horizontal
     * centroids in PRUNING
     */
    void CentroidsToAuxiliaryHorizontal(const size_t n_clusters) {
        Eigen::Map<MatrixR> hor_centroids(horizontal_centroids.get(), n_clusters, d);
        Eigen::Map<MatrixR> out_aux_centroids(
            partial_horizontal_centroids.get(), n_clusters, vertical_d
        );
        out_aux_centroids.noalias() = hor_centroids.leftCols(vertical_d);
    }

    /**
     * @brief Tune partial_d based on the average not-pruned percentage.
     *
     * A safe range for pruning is between 95% - 97% of vectors pruned (i.e., 3% - 5% not pruned).
     * - If avg_not_pruned_pct > 5% (i.e., less than 95% pruned), we reduce partial_d by
     * config.adjustment_factor_for_partial_d to be more aggressive in pruning
     * - If avg_not_pruned_pct < 3% (i.e., more than 97% pruned), we increase partial_d by
     * config.adjustment_factor_for_partial_d to be less aggressive
     * - partial_d is clamped between MIN_PARTIAL_D and vertical_d
     *
     * @param not_pruned_counts Buffer containing per-vector not-pruned counts
     * @param n_samples Number of X vectors
     * @param n_y Number of Y vectors (centroids)
     * @param partial_d_changed Output parameter: set to true if partial_d was changed
     * @return The computed average not-pruned percentage
     */
    float TunePartialD(
        const size_t* not_pruned_counts,
        size_t n_samples,
        size_t n_y,
        bool& partial_d_changed
    ) {
        float avg_not_pruned_pct = 0.0f;
        for (size_t i = 0; i < n_samples; ++i) {
            avg_not_pruned_pct += static_cast<float>(not_pruned_counts[i]);
        }
        avg_not_pruned_pct /= static_cast<float>(n_samples * n_y);

        uint32_t old_partial_d = partial_d;
        if (avg_not_pruned_pct > config.max_not_pruned_pct) {
            // Too many vectors not pruned (< max_not_pruned_pct pruned), need more GEMM dimensions
            // Increase partial_d by adjustment_factor_for_partial_d * 2
            auto increase =
                static_cast<uint32_t>(partial_d * config.adjustment_factor_for_partial_d * 2);
            partial_d = std::min(partial_d + std::max(increase, 1u), vertical_d);
        } else if (avg_not_pruned_pct < config.min_not_pruned_pct) {
            // Too few vectors not pruned (> min_not_pruned_pct pruned), can reduce GEMM dimensions
            // Decrease partial_d by adjustment_factor_for_partial_d
            auto decrease =
                static_cast<uint32_t>(partial_d * config.adjustment_factor_for_partial_d);
            partial_d = std::max(partial_d - std::max(decrease, 1u), MIN_PARTIAL_D);
        }
        partial_d_changed = (old_partial_d != partial_d);
        return avg_not_pruned_pct;
    }

    /**
     * @brief Computes the number of vectors to sample based on sampling_fraction.
     * To be conservative, we implement two heuristics:
     * - We sample at most max_points_per_cluster points per cluster (FAISS style)
     * - We sample at most sampling_fraction * n points (our style)
     * We return the minimum of the two.
     * @param n Total number of vectors
     * @return Number of vectors to sample
     */
    [[nodiscard]] virtual size_t GetNVectorsToSample(const size_t n, size_t n_clusters) const {
        if (config.sampling_fraction == 1.0) {
            return n;
        }
        auto samples_byn_clusters = n_clusters * config.max_points_per_cluster;
        auto samples_by_n =
            static_cast<size_t>(std::floor(static_cast<float>(n) * config.sampling_fraction));
        return std::min(samples_by_n, samples_byn_clusters);
    }

    /**
     * @brief Check if the core loop should stop early based on convergence criteria.
     *
     * Convergence is detected when either:
     * - Shift is below tolerance (shift < config.tol) or
     * - Recall hasn't improved by more than config.recall_tol in RECALL_CONVERGENCE_PATIENCE
     * consecutive iterations (when tracking recall)
     *
     * @param tracking_recall Whether recall is being tracked (n_queries > 0)
     * @param best_recall Reference to the best recall seen so far (updated if current is better)
     * @param iters_without_improvement Reference to counter of iterations without recall
     * improvement
     * @param iter_idx Current iteration index
     * @return true if training should stop, false otherwise
     */
    bool ShouldStopEarly(
        const bool tracking_recall,
        float& best_recall,
        size_t& iters_without_improvement,
        const size_t iter_idx
    ) {
        if (shift < config.tol) {
            if (config.verbose)
                std::cout << "Converged at iteration " << iter_idx + 1 << " (shift " << shift
                          << " < tol " << config.tol << ")" << std::endl;
            return true;
        }
        if (iter_idx > 0) {
            auto cost_delta = cost / prev_cost;
            if (cost_delta > 1 - config.tol) {
                if (config.verbose)
                    std::cout << "Converged at iteration " << iter_idx + 1 << " (cost "
                              << " improved by only " << 1 - cost_delta << ")" << std::endl;
                return true;
            }
        }
        if (tracking_recall) {
            float improvement = recall - best_recall;
            if (improvement > config.recall_tol) {
                best_recall = recall;
                iters_without_improvement = 0;
            } else {
                iters_without_improvement++;
                if (iters_without_improvement >= RECALL_CONVERGENCE_PATIENCE) {
                    if (config.verbose)
                        std::cout << "Converged at iteration " << iter_idx + 1 << " (recall "
                                  << recall << " hasn't improved by more than " << config.recall_tol
                                  << " in " << RECALL_CONVERGENCE_PATIENCE
                                  << " iterations, best: " << best_recall << ")" << std::endl;
                    return true;
                }
            }
        }
        return false;
    }

    /**
     * @brief Unrotate centroids to original space.
     * @param should_unrotate If true, unrotates centroids to original space, if false, returns
     * rotated centroids.
     * @return Centroids
     */
    std::vector<centroid_value_t> GetOutputCentroids(bool should_unrotate) {
        if (should_unrotate) {
            SKM_PROFILE_SCOPE("unrotator");
            std::vector<centroid_value_t> unrotated(n_clusters * d);
            pruner->Unrotate(horizontal_centroids.get(), unrotated.data(), n_clusters);
            return unrotated;
        }
        return std::vector<centroid_value_t>(
            horizontal_centroids.get(), horizontal_centroids.get() + n_clusters * d
        );
    }

    /**
     * @brief Normalizes centroids to unit length for inner product distance.
     *
     */
    void PostprocessCentroids(const size_t n_clusters) {
        auto horizontal_centroids_p = horizontal_centroids.get();
#pragma omp parallel for if (n_threads > 1) num_threads(n_threads)
        for (size_t i = 0; i < n_clusters; ++i) {
            auto horizontal_centroids_p_i = horizontal_centroids_p + i * d;
            float sum = 0.0f;
            for (size_t j = 0; j < d; ++j) {
                sum += horizontal_centroids_p_i[j] * horizontal_centroids_p_i[j];
            }
            float norm = 1.0f / std::sqrt(sum);
            for (size_t j = 0; j < d; ++j) {
                horizontal_centroids_p_i[j] *= norm;
            }
        }
    }

    /**
     * @brief Samples and optionally rotates vectors for training.
     *
     * Performs random sampling without replacement using shuffle technique,
     * and optionally rotates the sampled vectors using the pruner's rotation matrix.
     *
     * @tparam ROTATE Whether to apply rotation (default true)
     * @param data Input data matrix
     * @param out_buffer Output buffer for sampled (and optionally rotated) vectors
     * @param n Total number of input vectors
     * @param n_samples Number of vectors to sample
     */
    const vector_value_t* SampleAndRotateVectors(
        const vector_value_t* SKM_RESTRICT data,
        vector_value_t* SKM_RESTRICT out,
        const size_t n,
        const size_t n_samples,
        const bool rotate = true
    ) {
        // Intermediate buffer needed only when both sampling and rotating
        // (we have not yet implemented rotation in-place)
        std::vector<vector_value_t> samples_tmp;
        const vector_value_t* src_data = data;

        if (n_samples < n) {
            if (config.verbose)
                std::cout << "Sampling " << n_samples << " vectors" << std::endl;
            SKM_PROFILE_SCOPE("sampling");
            // Random sampling without replacement
            std::mt19937 rng(config.seed);
            sampled_indices.reset(new size_t[n]);
            for (size_t i = 0; i < n; ++i) {
                sampled_indices[i] = i;
            }
            std::shuffle(sampled_indices.get(), sampled_indices.get() + n, rng);
            if (rotate) {
                samples_tmp.reserve(n_samples * d);
                // Need intermediate buffer: sample first, then rotate
#pragma omp parallel for if (n_threads > 1) num_threads(n_threads)
                for (size_t i = 0; i < n_samples; ++i) {
                    memcpy(
                        static_cast<void*>(samples_tmp.data() + i * d),
                        static_cast<const void*>(data + sampled_indices[i] * d),
                        sizeof(vector_value_t) * d
                    );
                }
                src_data = samples_tmp.data();
            } else {
                // No rotation: copy directly into output buffer
#pragma omp parallel for if (n_threads > 1) num_threads(n_threads)
                for (size_t i = 0; i < n_samples; ++i) {
                    memcpy(
                        static_cast<void*>(out + i * d),
                        static_cast<const void*>(data + sampled_indices[i] * d),
                        sizeof(vector_value_t) * d
                    );
                }
                return out;
            }
        }
        if (config.verbose)
            std::cout << "Using " << n_samples << " vectors" << std::endl;

        // No rotation and no sampling: use the original data directly (avoid redundant memcpy)
        if (!rotate) {
            return data;
        }

        RotateOrCopy(src_data, out, n_samples, rotate);
        return out;
    }

    const size_t d;
    const size_t n_clusters;
    SuperKMeansConfig config;

    uint32_t n_threads;
    size_t n_samples = 0;
    uint32_t partial_d = 0; // d'

    // Iteration state
    bool trained = false;
    size_t n_split = 0;
    size_t centroids_to_explore = 0;
    uint32_t vertical_d = 0;
    float prev_cost = 0.0f;
    float cost = 0.0f;
    float shift = 0.0f;
    float recall = 0.0f;

    std::unique_ptr<pruner_t> pruner;

    // Centroids data (unoptimized space)
    // TODO(@lkuffo, high): 3 copies of the centroids? Can we do better?
    //    We can trivially avoid partial_horizontal_centroids by using the full horizontal
    //    centroids in PDXearch. We can also avoid prev_centroids if we dont care about the shift
    //    convergence check.
    std::unique_ptr<centroid_value_t[]> centroids;            // PDX-layout centroids
    std::unique_ptr<centroid_value_t[]> horizontal_centroids; // Row-major centroids
    std::unique_ptr<centroid_value_t[]> prev_centroids;       // Previous iteration centroids
    std::unique_ptr<centroid_value_t[]> partial_horizontal_centroids; // First vertical_d dimensions

    // Buffers for assignment and distance computation
    std::unique_ptr<distance_t[]> distances;
    std::unique_ptr<uint32_t[]> cluster_sizes;
    std::unique_ptr<vector_value_t[]> data_norms;
    std::unique_ptr<vector_value_t[]> centroid_norms;
    std::unique_ptr<size_t[]> sampled_indices;

    // Buffers for ground truth and recall computation
    std::unique_ptr<uint32_t[]> gt_assignments;
    std::unique_ptr<distance_t[]> gt_distances;
    std::unique_ptr<distance_t[]> query_norms;
    std::unique_ptr<distance_t[]> tmp_distances_buffer;
    std::unique_ptr<uint32_t[]> promising_centroids;
    std::unique_ptr<distance_t[]> recall_distances;

  public:
    std::unique_ptr<uint32_t[]> assignments;
    std::vector<SuperKMeansIterationStats> iteration_stats;
};
} // namespace skmeans
