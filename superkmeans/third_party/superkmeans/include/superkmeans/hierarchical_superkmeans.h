#pragma once

#include "superkmeans/pdx/utils.h"
#include "superkmeans/superkmeans.h"

namespace skmeans {

/**
 * @brief Configuration parameters for Hierarchical SuperKMeans clustering.
 */
struct HierarchicalSuperKMeansConfig : SuperKMeansConfig {
    uint32_t iters_mesoclustering = 3;
    uint32_t iters_fineclustering = 5;
    uint32_t iters_refinement = 0; // Refinement iteration is not needed to achieve good recall

    HierarchicalSuperKMeansConfig() { sampling_fraction = 1.0f; }
};

/**
 * @brief Statistics for HierarchicalSuperKMeans clustering.
 */
struct HierarchicalSuperKMeansIterationStats {
    std::vector<SuperKMeansIterationStats> mesoclustering_iteration_stats;
    std::vector<SuperKMeansIterationStats> refinement_iteration_stats;
    std::vector<SuperKMeansIterationStats> fineclustering_iteration_stats;
};

template <Quantization q = Quantization::f32, DistanceFunction alpha = DistanceFunction::l2>
class HierarchicalSuperKMeans : public SuperKMeans<q, alpha> {
    using typename SuperKMeans<q, alpha>::centroid_value_t;
    using typename SuperKMeans<q, alpha>::vector_value_t;
    using typename SuperKMeans<q, alpha>::distance_t;
    using typename SuperKMeans<q, alpha>::MatrixR;
    using typename SuperKMeans<q, alpha>::VectorR;
    using typename SuperKMeans<q, alpha>::pruner_t;
    using typename SuperKMeans<q, alpha>::layout_t;
    using typename SuperKMeans<q, alpha>::batch_computer;

  public:
    /**
     * @brief Constructor with custom configuration
     */
    HierarchicalSuperKMeans(
        size_t n_clusters,
        size_t dimensionality,
        const HierarchicalSuperKMeansConfig& config
    )
        : SuperKMeans<q, alpha>(n_clusters, dimensionality, config), hierarchical_config(config) {
        this->pruner = std::make_unique<pruner_t>(
            dimensionality, HIERARCHICAL_PRUNER_INITIAL_THRESHOLD, this->config.seed
        );
        SKMEANS_ENSURE_POSITIVE(config.iters_mesoclustering);
        SKMEANS_ENSURE_POSITIVE(config.iters_fineclustering);

        static_cast<SuperKMeansConfig&>(hierarchical_config) = this->config;
        hierarchical_config.sampling_fraction = config.sampling_fraction;

        if (n_clusters <= 128) {
            if (!this->hierarchical_config.suppress_warnings) {
                std::cout
                    << "WARNING: n_clusters <= 128 is not recommended for HierarchicalSuperKMeans. "
                       "Consider using at least 128 clusters."
                    << std::endl;
            }
        }
    }

    /**
     * @brief Default constructor
     */
    HierarchicalSuperKMeans(size_t n_clusters, size_t dimensionality)
        : HierarchicalSuperKMeans(n_clusters, dimensionality, HierarchicalSuperKMeansConfig{}) {}

    /**
     * @brief Run hierarchical k-means clustering to determine centroids
     * We don't support Early Termination by Recall here.
     * queries and n_queries are ignored. But we keep the function signature for compatibility.
     *
     * @param data Pointer to the data matrix (row-major, n × d)
     * @param n Number of points (rows) in the data matrix
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
        if (this->trained) {
            throw std::runtime_error("The clustering has already been trained");
        }
        if (n_queries > 0) {
            if (!this->hierarchical_config.suppress_warnings) {
                std::cout << "WARNING: Early Termination by Recall is not supported in "
                             "HierarchicalSuperKMeans"
                          << std::endl;
            }
        }
        n_mesoclusters = GetNMesoclusters(this->n_clusters);
        hierarchical_iteration_stats.fineclustering_iteration_stats.clear();
        hierarchical_iteration_stats.mesoclustering_iteration_stats.clear();
        hierarchical_iteration_stats.refinement_iteration_stats.clear();
        if (n < this->n_clusters) {
            throw std::runtime_error(
                "The number of points should be at least as large as the number of clusters"
            );
        }
        const vector_value_t* SKM_RESTRICT data_p = data;
        this->n_samples = this->GetNVectorsToSample(n, this->n_clusters);
        if (this->n_samples < this->n_clusters) {
            throw std::runtime_error(
                "Not enough samples to train. Try increasing the sampling_fraction or "
                "max_points_per_cluster"
            );
        }
        {
            SKM_PROFILE_SCOPE("allocator");
            // Buffers to concatenate each fineclustering assignments and centroids
            this->final_assignments.reset(new uint32_t[this->n_samples]);
            this->final_centroids.reset(new centroid_value_t[this->n_clusters * this->d]);
            // These buffers are reused for all three phases
            this->centroids.reset(new centroid_value_t[this->n_clusters * this->d]);
            this->horizontal_centroids.reset(new centroid_value_t[this->n_clusters * this->d]);
            this->prev_centroids.reset(new centroid_value_t[this->n_clusters * this->d]);
            this->cluster_sizes.reset(new uint32_t[this->n_clusters]);
            this->assignments.reset(new uint32_t[n]);
            this->distances.reset(new distance_t[n]);
            this->data_norms.reset(new vector_value_t[this->n_samples]);
            this->centroid_norms.reset(new vector_value_t[this->n_clusters]);
        }
        std::vector<distance_t> tmp_distances_buf;
        tmp_distances_buf.reserve(X_BATCH_SIZE * Y_BATCH_SIZE);
        this->vertical_d = PDXLayout<q, alpha>::GetDimensionSplit(this->d).vertical_d;
        this->partial_horizontal_centroids.reset(
            new centroid_value_t[this->n_clusters * this->vertical_d]
        );

        this->partial_d = std::max<uint32_t>(MIN_PARTIAL_D, this->vertical_d / 2);

        if (this->partial_d > this->vertical_d) {
            this->partial_d = this->vertical_d;
        }
        auto initial_partial_d = this->partial_d;

        if (this->hierarchical_config.verbose) {
            std::cout << "Front dimensions (d') = " << this->partial_d << std::endl;
            std::cout << "Trailing dimensions (d'') = " << this->d - this->vertical_d << std::endl;
        }

        //
        // MESOCLUSTERING
        //
        TicToc timer_mesoclustering;
        timer_mesoclustering.Tic();
        if (this->hierarchical_config.verbose) {
            std::cout << "\n=== PHASE 1: MESOCLUSTERING (k=" << n_mesoclusters
                      << " clusters) ===" << std::endl;
        }
        auto centroids_pdx_wrapper = this->GenerateCentroids(
            data_p, this->n_samples, n_mesoclusters, !this->hierarchical_config.data_already_rotated
        );
        if (this->hierarchical_config.verbose) {
            std::cout << "Sampling data..." << std::endl;
        }
        // Samples for both mesoclustering and fineclustering
        std::vector<vector_value_t> data_samples_buffer;
        data_samples_buffer.reserve(this->n_samples * this->d);
        auto data_to_cluster = this->SampleAndRotateVectors(
            data_p,
            data_samples_buffer.data(),
            n,
            this->n_samples,
            !this->hierarchical_config.data_already_rotated
        );
        auto initialn_samples = this->n_samples;
        this->RotateOrCopy(
            this->horizontal_centroids.get(),
            this->prev_centroids.get(),
            n_mesoclusters,
            !this->hierarchical_config.data_already_rotated
        );
        this->GetL2NormsRowMajor(data_to_cluster, this->n_samples, this->data_norms.get());
        this->GetL2NormsRowMajor(
            this->prev_centroids.get(), n_mesoclusters, this->centroid_norms.get()
        );

        // Buffers for RunIteration (needed for function signature even if unused in GEMM-only mode)
        std::vector<vector_value_t> centroids_partial_norms;
        centroids_partial_norms.reserve(this->n_clusters);
        std::vector<size_t> not_pruned_counts;
        not_pruned_counts.reserve(this->n_samples);

        // Save full norms before the loop (independent of iteration work)
        {
            SKM_PROFILE_SCOPE("allocator");
            immutable_data_norms.reset(new vector_value_t[this->n_samples]);
            memcpy(
                immutable_data_norms.get(),
                this->data_norms.get(),
                sizeof(vector_value_t) * this->n_samples
            );
        }

        bool always_gemm_only = this->d < DIMENSION_THRESHOLD_FOR_PRUNING ||
                                this->hierarchical_config.use_blas_only ||
                                n_mesoclusters <= N_CLUSTERS_THRESHOLD_FOR_PRUNING;
        bool partial_norms_computed = false;
        float best_recall = 0.0f;
        size_t iters_without_improvement = 0;

        for (size_t iter_idx = 0; iter_idx < this->hierarchical_config.iters_mesoclustering;
             ++iter_idx) {
            bool use_gemm_only = (iter_idx == 0) || always_gemm_only;
            if (!use_gemm_only && !partial_norms_computed) {
                this->GetPartialL2NormsRowMajor(
                    data_to_cluster, this->n_samples, this->data_norms.get(), this->partial_d
                );
                partial_norms_computed = true;
            }
            if (use_gemm_only) {
                this->template RunIteration<true>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    centroids_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    nullptr, // queries
                    0,       // n_queries
                    this->n_samples,
                    n_mesoclusters,
                    iter_idx,
                    iter_idx == 0,
                    this->hierarchical_iteration_stats.mesoclustering_iteration_stats
                );
            } else {
                this->template RunIteration<false>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    centroids_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    nullptr, // queries
                    0,       // n_queries
                    this->n_samples,
                    n_mesoclusters,
                    iter_idx,
                    false,
                    this->hierarchical_iteration_stats.mesoclustering_iteration_stats
                );
            }
            if (this->hierarchical_config.early_termination &&
                this->ShouldStopEarly(false, best_recall, iters_without_improvement, iter_idx)) {
                break;
            }
        }
        timer_mesoclustering.Toc();

        {
            SKM_PROFILE_SCOPE("allocator");
            mesoclusters_sizes.assign(
                this->cluster_sizes.get(), this->cluster_sizes.get() + n_mesoclusters
            );
            mesoclusters_assignments.assign(
                this->assignments.get(), this->assignments.get() + this->n_samples
            );
        }

        // Build partitioned index for efficient mesocluster compaction
        std::vector<size_t> mesocluster_indices_flat;
        mesocluster_indices_flat.resize(this->n_samples);
        std::vector<size_t> mesocluster_offsets;
        mesocluster_offsets.resize(n_mesoclusters + 1);
        {
            SKM_PROFILE_SCOPE("compact_indices");
            mesocluster_offsets[0] = 0;
            for (size_t k = 0; k < n_mesoclusters; ++k) {
                mesocluster_offsets[k + 1] = mesocluster_offsets[k] + mesoclusters_sizes[k];
            }
            std::vector<size_t> next_to_write_index = mesocluster_offsets;
            for (size_t i = 0; i < this->n_samples; ++i) {
                size_t cluster_id = mesoclusters_assignments[i];
                mesocluster_indices_flat[next_to_write_index[cluster_id]++] = i;
            }
        }

        //
        // FINE-CLUSTERING
        // Each mesocluster is re-clustered sequentially
        // Potential improvement: Doing 2 only-GEMM iterations and delegate the rest to GEMM+PRUNING
        // seems an interesting idea. However, we need to evaluate this with a larger dataset (+100M
        // vectors)
        //
        if (this->hierarchical_config.verbose) {
            std::cout << "\n=== PHASE 2: FINE-CLUSTERING (subdividing " << n_mesoclusters
                      << " mesoclusters into total " << this->n_clusters
                      << " clusters) ===" << std::endl;
        }
        // Calculate proportional allocation of fine clusters per mesocluster
        auto fine_clusters_nums = ArrangeFineClusters(
            this->n_clusters, n_mesoclusters, this->n_samples, mesoclusters_sizes
        );

        TicToc timer_fineclustering;
        timer_fineclustering.Tic();

        size_t max_mesocluster_size = *std::max_element(
            this->cluster_sizes.get(), this->cluster_sizes.get() + n_mesoclusters
        );
        std::vector<vector_value_t> mesocluster_buffer(max_mesocluster_size * this->d);
        std::vector<uint32_t> assignments_indirection_buffer(max_mesocluster_size);
        size_t fineclusters_offset = 0;
        for (size_t k = 0; k < n_mesoclusters; ++k) {
            size_t n_fineclusters = fine_clusters_nums[k];
            if (n_fineclusters == 0) {
                continue;
            }
            this->partial_d = initial_partial_d;

            auto mesocluster_size = mesoclusters_sizes[k];
            // auto points_per_finecluster = static_cast<float>(mesocluster_size) /
            // static_cast<float>(n_fineclusters);
            this->n_samples = mesocluster_size;
            CompactMesoclusterToBuffer(
                mesocluster_size,
                data_to_cluster,
                mesocluster_buffer.data(),
                assignments_indirection_buffer.data(),
                mesocluster_indices_flat.data() + mesocluster_offsets[k]
            );
            auto mesocluster_data_to_cluster = mesocluster_buffer.data();
            auto mesocluster_centroids_pdx_wrapper = this->GenerateCentroids(
                mesocluster_data_to_cluster, mesocluster_size, n_fineclusters, false
            );
            // Copy centroids to prev_centroids for use in the first RunIteration
            // (is_first_iter=true skips the swap, so prev_centroids must be populated)
            memcpy(
                this->prev_centroids.get(),
                this->horizontal_centroids.get(),
                sizeof(centroid_value_t) * n_fineclusters * this->d
            );
            this->GetL2NormsRowMajor(
                this->prev_centroids.get(), n_fineclusters, this->centroid_norms.get()
            );

            bool fine_always_gemm_only = this->d < DIMENSION_THRESHOLD_FOR_PRUNING ||
                                         this->hierarchical_config.use_blas_only ||
                                         n_fineclusters <= N_CLUSTERS_THRESHOLD_FOR_PRUNING;
            bool fine_partial_norms_computed = false;
            float fine_best_recall = 0.0f;
            iters_without_improvement = 0;

            for (size_t fine_iter_idx = 0;
                 fine_iter_idx < this->hierarchical_config.iters_fineclustering;
                 ++fine_iter_idx) {
                bool use_gemm_only = (fine_iter_idx == 0) || fine_always_gemm_only;
                if (!use_gemm_only && !fine_partial_norms_computed) {
                    this->GetPartialL2NormsRowMajor(
                        mesocluster_data_to_cluster,
                        this->n_samples,
                        this->data_norms.get(),
                        this->partial_d
                    );
                    fine_partial_norms_computed = true;
                }
                if (use_gemm_only) {
                    this->template RunIteration<true>(
                        mesocluster_data_to_cluster,
                        tmp_distances_buf.data(),
                        mesocluster_centroids_pdx_wrapper,
                        centroids_partial_norms,
                        not_pruned_counts,
                        nullptr, // queries
                        0,       // n_queries
                        this->n_samples,
                        n_fineclusters,
                        fine_iter_idx,
                        fine_iter_idx == 0,
                        this->hierarchical_iteration_stats.fineclustering_iteration_stats
                    );
                } else {
                    this->template RunIteration<false>(
                        mesocluster_data_to_cluster,
                        tmp_distances_buf.data(),
                        mesocluster_centroids_pdx_wrapper,
                        centroids_partial_norms,
                        not_pruned_counts,
                        nullptr, // queries
                        0,       // n_queries
                        this->n_samples,
                        n_fineclusters,
                        fine_iter_idx,
                        false,
                        this->hierarchical_iteration_stats.fineclustering_iteration_stats
                    );
                }
                if (this->hierarchical_config.early_termination &&
                    this->ShouldStopEarly(
                        false, fine_best_recall, iters_without_improvement, fine_iter_idx
                    )) {
                    break;
                }
            }

            GetTrueAssignmentsFromIndirectionBuffer(
                assignments_indirection_buffer.data(),
                final_assignments.get(),
                this->assignments.get(),
                mesocluster_size,
                fineclusters_offset
            );

            // We move the resulting centroids from this fineclustering to the final buffer of
            // centroids
            {
                SKM_PROFILE_SCOPE("copy_fine_centroids");
                memcpy(
                    static_cast<void*>(final_centroids.get() + fineclusters_offset * this->d),
                    static_cast<void*>(this->horizontal_centroids.get()),
                    sizeof(centroid_value_t) * n_fineclusters * this->d
                );
            }
            fineclusters_offset += n_fineclusters;
        }
        timer_fineclustering.Toc();

        // Now we move to the last refinement phase in which we perform clustering with all
        // n_clusters. Recall our initial buffers for centroids have enough space for n_clusters.
        if (this->hierarchical_config.verbose) {
            std::cout << "\n=== PHASE 3: REFINEMENT (fine-tuning all " << this->n_clusters
                      << " clusters) ===" << std::endl;
        }
        this->n_samples = initialn_samples;

        // In the refinement phase, we use an even smaller partial d (around 8% of d) because the
        // clusters are already well-formed, and pruning rate is expected to be high.
        this->partial_d = std::max<uint32_t>(MIN_PARTIAL_D, this->vertical_d / 3);

        // We just transfer the state of centroids to the proper class variables, no rotation.
        auto final_refinement_pdx_wrapper = SetupCentroids(final_centroids.get(), this->n_clusters);

        // (RunIteration with is_first_iter=false will swap horizontal_centroids and
        // prev_centroids) Copy final_centroids to prev_centroids so the swap in RunIteration
        // works correctly We could avoid this copies by managing an offset on assignments and
        // centroids in the core functions of SuperKMeans. But this would just complicate the code.
        {
            SKM_PROFILE_SCOPE("copy_final_centroids_and_assignments");
            memcpy(
                static_cast<void*>(this->prev_centroids.get()),
                static_cast<void*>(final_centroids.get()),
                sizeof(centroid_value_t) * this->n_clusters * this->d
            );
            memcpy(
                static_cast<void*>(this->assignments.get()),
                static_cast<void*>(final_assignments.get()),
                sizeof(uint32_t) * this->n_samples
            );
        }
        this->GetL2NormsRowMajor(
            this->prev_centroids.get(), this->n_clusters, this->centroid_norms.get()
        );

        TicToc timer_refinement;
        timer_refinement.Tic();

        // Refinement iterations
        bool refinement_always_gemm_only = this->d < DIMENSION_THRESHOLD_FOR_PRUNING ||
                                           this->n_clusters <= N_CLUSTERS_THRESHOLD_FOR_PRUNING;
        bool refinement_partial_norms_computed = false;
        for (size_t refinement_iter_idx = 0;
             refinement_iter_idx < this->hierarchical_config.iters_refinement;
             ++refinement_iter_idx) {
            if (!refinement_always_gemm_only && !refinement_partial_norms_computed) {
                // TODO(@lkuffo, high): The only reason I need to compute the data norms (again)
                //   is because we are using the same this->data_norms.get() buffer in the
                //   fineclustering, which replaces the norms that I already calculated before and
                //   put in this buffer.
                this->GetPartialL2NormsRowMajor(
                    data_to_cluster, this->n_samples, this->data_norms.get(), this->partial_d
                );
                refinement_partial_norms_computed = true;
            }
            if (refinement_always_gemm_only) {
                this->template RunIteration<true>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    final_refinement_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    nullptr, // queries
                    0,       // n_queries
                    this->n_samples,
                    this->n_clusters,
                    refinement_iter_idx,
                    false,
                    this->hierarchical_iteration_stats.refinement_iteration_stats
                );
            } else {
                this->template RunIteration<false>(
                    data_to_cluster,
                    tmp_distances_buf.data(),
                    final_refinement_pdx_wrapper,
                    centroids_partial_norms,
                    not_pruned_counts,
                    nullptr, // queries
                    0,       // n_queries
                    this->n_samples,
                    this->n_clusters,
                    refinement_iter_idx,
                    false,
                    this->hierarchical_iteration_stats.refinement_iteration_stats
                );
            }
        }
        timer_refinement.Toc();

        if (this->hierarchical_config.verbose) {
            std::cout << "Mesoclustering time: " << timer_mesoclustering.GetMilliseconds() << " ms"
                      << std::endl;
            std::cout << "Fineclustering time: " << timer_fineclustering.GetMilliseconds() << " ms"
                      << std::endl;
            std::cout << "Refinement time: " << timer_refinement.GetMilliseconds() << " ms"
                      << std::endl;
            std::cout << "Total time: "
                      << timer_mesoclustering.GetMilliseconds() +
                             timer_fineclustering.GetMilliseconds() +
                             timer_refinement.GetMilliseconds()
                      << " ms" << std::endl;
        }
        this->trained = true;

        // TODO(@lkuffo, high): If unrotate_centroids is false, this computes incorrect assignments
        //   because it's using unrotated data with rotated output_centroids
        auto output_centroids =
            this->GetOutputCentroids(this->hierarchical_config.unrotate_centroids);
        if (this->hierarchical_config.verbose) {
            Profiler::Get().PrintHierarchical();
        }
        return output_centroids;
    }

    /**
     * @brief Calculate the number of mesoclusters for a given number of clusters
     *
     * @param n_clusters Total number of clusters
     * @return Number of mesoclusters
     */
    static size_t GetNMesoclusters(const size_t n_clusters) {
        return static_cast<size_t>(std::round(std::sqrt(n_clusters)));
    }

    /**
     * @brief Computes the number of vectors to sample based on sampling_fraction.
     *
     * @param n Total number of vectors
     * @return Number of vectors to sample
     */
    [[nodiscard]] size_t GetNVectorsToSample(const size_t n, size_t n_clusters) const override {
        if (this->hierarchical_config.sampling_fraction == 1.0) {
            return n;
        }
        auto samples_by_n = static_cast<size_t>(
            std::floor(static_cast<double>(n) * this->hierarchical_config.sampling_fraction)
        );
        return samples_by_n;
    }

    /**
     * @brief Override SplitClusters with more aggressive balancing similar to cuVS
     *
     * This version not only handles empty clusters but also actively rebalances
     * small clusters (those below a threshold) by moving their centers toward
     * points from larger clusters.
     *
     * @param n_samples Total number of samples
     * @param n_clusters Number of clusters
     */
    void SplitClusters(const size_t n_samples, const size_t n_clusters) override {
        constexpr float CENTER_ADJUSTMENT_WEIGHT =
            7.0f; // Weight for current center in weighted average
        constexpr float BALANCING_THRESHOLD =
            0.25f; // Clusters smaller than 25% of average are adjusted

        this->n_split = 0;
        std::mt19937 rng(this->config.seed);
        auto horizontal_centroids_p = this->horizontal_centroids.get();

        size_t average_size = n_samples / n_clusters;
        size_t threshold_size =
            static_cast<size_t>(static_cast<float>(average_size) * BALANCING_THRESHOLD);
        {
            SKM_PROFILE_SCOPE("consolidate/empty");
            for (size_t ci = 0; ci < n_clusters; ci++) {
                if (this->cluster_sizes[ci] == 0) {
                    size_t cj;
                    for (cj = 0; true; cj = (cj + 1) % n_clusters) {
                        float p = (this->cluster_sizes[cj] - 1.0f) /
                                  static_cast<float>(n_samples - n_clusters);
                        float r = std::uniform_real_distribution<float>(0, 1)(rng);
                        if (r < p) {
                            break;
                        }
                    }

                    memcpy(
                        (void*) (horizontal_centroids_p + ci * this->d),
                        (void*) (horizontal_centroids_p + cj * this->d),
                        sizeof(centroid_value_t) * this->d
                    );

                    // Small symmetric perturbation
                    for (size_t j = 0; j < this->d; j++) {
                        if (j % 2 == 0) {
                            horizontal_centroids_p[ci * this->d + j] *=
                                1.0f + CENTROID_PERTURBATION_EPS;
                            horizontal_centroids_p[cj * this->d + j] *=
                                1.0f - CENTROID_PERTURBATION_EPS;
                        } else {
                            horizontal_centroids_p[ci * this->d + j] *=
                                1.0f - CENTROID_PERTURBATION_EPS;
                            horizontal_centroids_p[cj * this->d + j] *=
                                1.0f + CENTROID_PERTURBATION_EPS;
                        }
                    }

                    // Assume even split of the cluster
                    this->cluster_sizes[ci] = this->cluster_sizes[cj] / 2;
                    this->cluster_sizes[cj] -= this->cluster_sizes[ci];
                    this->n_split++;
                }
            }
        }

        // Adjust small clusters (cuVS-style balancing)
        // Pick large clusters with probability proportional to their size
        {
            SKM_PROFILE_SCOPE("consolidate/balancing");
            for (size_t ci = 0; ci < n_clusters; ci++) {
                size_t csize = this->cluster_sizes[ci];
                if (csize == 0 || csize > threshold_size)
                    continue;

                // Find a large cluster with probability proportional to its size
                size_t large_cluster_idx;
                for (large_cluster_idx = 0; true;
                     large_cluster_idx = (large_cluster_idx + 1) % n_clusters) {
                    size_t large_size = this->cluster_sizes[large_cluster_idx];
                    if (large_size < average_size)
                        continue;
                    // Probability proportional to how much larger this cluster is than average
                    float p =
                        static_cast<float>(large_size - average_size + 1) /
                        static_cast<float>(n_samples - average_size * n_clusters + n_clusters);
                    float r = std::uniform_real_distribution<float>(0, 1)(rng);
                    if (r < p) {
                        break; // Found our cluster to be split
                    }
                }

                // Adjust the center of the selected smaller cluster to gravitate towards
                // a sample from the selected larger cluster.
                // Weight of the current center for the weighted average.
                // We dump it for anomalously small clusters, but keep constant otherwise.
                float wc = std::min(static_cast<float>(csize), CENTER_ADJUSTMENT_WEIGHT);
                float wd = 1.0f; // Weight for the datapoint used to shift the center.
                for (size_t j = 0; j < this->d; j++) {
                    float val = 0.0f;
                    val += wc * horizontal_centroids_p[ci * this->d + j];
                    val += wd * horizontal_centroids_p[large_cluster_idx * this->d + j];
                    val /= (wc + wd);
                    horizontal_centroids_p[ci * this->d + j] = val;
                }

                this->n_split++;
            }
        }
    }

    /*
     * Compact data assigned to a mesocluster in mesocluster_buffer using precomputed indices
     * Data is already rotated, so we just have to copy
     * Additionally, we have to copy their norms in a sequential buffer to not recompute them
     */
    void CompactMesoclusterToBuffer(
        const size_t mesocluster_size,
        const vector_value_t* SKM_RESTRICT data,
        vector_value_t* SKM_RESTRICT mesocluster_buffer,
        uint32_t* SKM_RESTRICT assignments_indirection_buffer,
        const size_t* SKM_RESTRICT mesocluster_indices
    ) {
        SKM_PROFILE_SCOPE("compact_mesocluster");
#pragma omp parallel for if (this->n_threads > 1) num_threads(this->n_threads)
        for (size_t j = 0; j < mesocluster_size; ++j) {
            size_t i = mesocluster_indices[j];
            this->data_norms[j] = immutable_data_norms[i];
            assignments_indirection_buffer[j] = i;
            memcpy(
                static_cast<void*>(mesocluster_buffer + j * this->d),
                static_cast<const void*>(data + i * this->d),
                sizeof(vector_value_t) * this->d
            );
        }
    }

    /**
     * @brief Arrange fine clusters proportionally to mesocluster sizes
     *
     * Allocates the total number of clusters across mesoclusters proportionally
     * to their sizes, ensuring balanced distribution.
     *
     * @param n_clusters Total number of fine clusters to distribute
     * @param n_mesoclusters Number of mesoclusters
     * @param n_samples Total number of samples
     * @param mesocluster_sizes Sizes of each mesocluster
     * @return Vector of fine cluster counts per mesocluster
     */
    std::vector<size_t> ArrangeFineClusters(
        size_t n_clusters,
        size_t n_mesoclusters,
        size_t n_samples,
        const std::vector<uint32_t>& mesocluster_sizes
    ) {
        SKM_PROFILE_SCOPE("arrange_fine_clusters");
        std::vector<size_t> fine_clusters_nums(n_mesoclusters);

        size_t n_clusters_remaining = n_clusters;
        size_t n_nonempty_mesoclusters_remaining = 0;
        for (size_t i = 0; i < n_mesoclusters; ++i) {
            if (mesocluster_sizes[i] > 0) {
                n_nonempty_mesoclusters_remaining++;
            }
        }

        size_t n_samples_remaining = n_samples;
        for (size_t i = 0; i < n_mesoclusters; ++i) {
            if (i < n_mesoclusters - 1) {
                // Handle empty mesoclusters
                if (mesocluster_sizes[i] == 0) {
                    fine_clusters_nums[i] = 0;
                } else {
                    n_nonempty_mesoclusters_remaining--;
                    double proportion =
                        static_cast<double>(n_clusters_remaining * mesocluster_sizes[i]) /
                        static_cast<double>(n_samples_remaining);
                    size_t allocated = static_cast<size_t>(std::lround(proportion));
                    allocated = std::min(
                        allocated, n_clusters_remaining - n_nonempty_mesoclusters_remaining
                    );
                    fine_clusters_nums[i] = std::max(allocated, size_t{1});
                }
            } else {
                // Last mesocluster gets all remaining clusters
                fine_clusters_nums[i] = n_clusters_remaining;
            }
            n_clusters_remaining -= fine_clusters_nums[i];
            n_samples_remaining -= mesocluster_sizes[i];
        }

        return fine_clusters_nums;
    }

    void GetTrueAssignmentsFromIndirectionBuffer(
        const uint32_t* SKM_RESTRICT assignments_indirection_buffer,
        uint32_t* SKM_RESTRICT output_assignments,
        const uint32_t* SKM_RESTRICT input_assignments,
        const size_t n_samples_in_mesocluster,
        const size_t cluster_id_offset
    ) {
        SKM_PROFILE_SCOPE("get_indirection_assignments");
        for (size_t i = 0; i < n_samples_in_mesocluster; ++i) {
            size_t original_idx = assignments_indirection_buffer[i];
            uint32_t local_cluster_id = input_assignments[i];
            // Assignments in fineclustering are from 0 to n_fineclusters-1
            // We need to add the offset to put the global cluster id in the final assignments
            uint32_t global_cluster_id = local_cluster_id + cluster_id_offset;
            output_assignments[original_idx] = global_cluster_id;
        }
    }

    /**
     * @brief Setup centroids to be used for the refinement clustering phase
     *
     * @param centroids Centroids to setup
     * @param n_clusters Number of centroids to setupt
     * @return PDXLayout wrapper for the centroids
     */
    PDXLayout<q, alpha> SetupCentroids(
        const centroid_value_t* SKM_RESTRICT centroids,
        const size_t n_clusters
    ) {
        SKM_PROFILE_SCOPE("consolidate");
        memcpy(
            (void*) (this->horizontal_centroids.get()),
            (void*) (centroids),
            sizeof(centroid_value_t) * n_clusters * this->d
        );
        {
            SKM_PROFILE_SCOPE("consolidate/pdxify");
            PDXLayout<q, alpha>::template PDXify<false>(
                this->horizontal_centroids.get(), this->centroids.get(), n_clusters, this->d
            );
        }
        //! We wrap centroids and partial_horizontal_centroids in the PDXLayout wrapper
        //! Any updates to these objects is reflected in the PDXLayout
        auto pdx_centroids = PDXLayout<q, alpha>(
            this->centroids.get(),
            *this->pruner,
            n_clusters,
            this->d,
            this->partial_horizontal_centroids.get()
        );
        this->CentroidsToAuxiliaryHorizontal(n_clusters);
        return pdx_centroids;
    }

    size_t n_mesoclusters = 0;

    std::vector<uint32_t> mesoclusters_assignments;
    std::vector<uint32_t> mesoclusters_sizes;
    std::unique_ptr<uint32_t[]> final_assignments;
    std::unique_ptr<vector_value_t[]> immutable_data_norms;
    std::unique_ptr<centroid_value_t[]> final_centroids;
    HierarchicalSuperKMeansConfig hierarchical_config;
    HierarchicalSuperKMeansIterationStats hierarchical_iteration_stats;
};

} // namespace skmeans
