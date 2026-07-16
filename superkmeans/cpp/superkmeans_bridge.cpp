#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <exception>
#include <new>
#include <string>
#include <vector>

#include "superkmeans/hierarchical_superkmeans.h"

struct SkmHierConfig {
    uint32_t iters;
    float sampling_fraction;
    uint32_t max_points_per_cluster;
    uint32_t n_threads;
    uint32_t seed;
    uint8_t use_blas_only;
    float tol;
    float recall_tol;
    uint8_t early_termination;
    uint8_t sample_queries;
    size_t objective_k;
    float ann_explore_fraction;
    float min_not_pruned_pct;
    float max_not_pruned_pct;
    float adjustment_factor_for_partial_d;
    uint8_t unrotate_centroids;
    uint8_t verbose;
    uint8_t angular;
    uint8_t suppress_warnings;
    uint8_t data_already_rotated;
    uint32_t iters_mesoclustering;
    uint32_t iters_fineclustering;
    uint32_t iters_refinement;
};

struct SkmIterationStats {
    size_t iteration;
    float objective;
    float shift;
    size_t split;
    float recall;
    float not_pruned_pct;
    uint32_t partial_d;
    uint8_t is_gemm_only;
};

class HierarchicalKMeansBridge
    : public skmeans::
          HierarchicalSuperKMeans<skmeans::Quantization::f32, skmeans::DistanceFunction::l2> {
  public:
    using skmeans::HierarchicalSuperKMeans<
        skmeans::Quantization::f32,
        skmeans::DistanceFunction::l2>::HierarchicalSuperKMeans;

    size_t n_samples_value() const {
        return this->n_samples;
    }
};

using HierarchicalKMeans = HierarchicalKMeansBridge;

struct SkmHierHandle {
    HierarchicalKMeans* ptr;
    std::string last_error;
};

struct SkmClusterBalanceStats {
    float mean;
    float geometric_mean;
    float stdev;
    float cv;
    size_t min;
    size_t max;
};

static void set_error(char** out_error, const std::string& message) {
    if (!out_error) {
        return;
    }
    auto* buffer = static_cast<char*>(std::malloc(message.size() + 1));
    if (!buffer) {
        *out_error = nullptr;
        return;
    }
    std::memcpy(buffer, message.c_str(), message.size() + 1);
    *out_error = buffer;
}

static void* copy_to_heap(const void* source, size_t bytes) {
    if (bytes == 0) {
        return nullptr;
    }
    auto* output = std::malloc(bytes);
    if (!output) {
        throw std::bad_alloc();
    }
    std::memcpy(output, source, bytes);
    return output;
}

static skmeans::HierarchicalSuperKMeansConfig to_cpp_config(const SkmHierConfig* input) {
    skmeans::HierarchicalSuperKMeansConfig config;
    config.iters = input->iters;
    config.sampling_fraction = input->sampling_fraction;
    config.max_points_per_cluster = input->max_points_per_cluster;
    config.n_threads = input->n_threads;
    config.seed = input->seed;
    config.use_blas_only = input->use_blas_only != 0;
    config.tol = input->tol;
    config.recall_tol = input->recall_tol;
    config.early_termination = input->early_termination != 0;
    config.sample_queries = input->sample_queries != 0;
    config.objective_k = input->objective_k;
    config.ann_explore_fraction = input->ann_explore_fraction;
    config.min_not_pruned_pct = input->min_not_pruned_pct;
    config.max_not_pruned_pct = input->max_not_pruned_pct;
    config.adjustment_factor_for_partial_d = input->adjustment_factor_for_partial_d;
    config.unrotate_centroids = input->unrotate_centroids != 0;
    config.verbose = input->verbose != 0;
    config.angular = input->angular != 0;
    config.suppress_warnings = input->suppress_warnings != 0;
    config.data_already_rotated = input->data_already_rotated != 0;
    config.iters_mesoclustering = input->iters_mesoclustering;
    config.iters_fineclustering = input->iters_fineclustering;
    config.iters_refinement = input->iters_refinement;
    return config;
}

static SkmHierConfig to_c_config(const skmeans::HierarchicalSuperKMeansConfig& input) {
    return SkmHierConfig{
        input.iters,
        input.sampling_fraction,
        input.max_points_per_cluster,
        input.n_threads,
        input.seed,
        static_cast<uint8_t>(input.use_blas_only),
        input.tol,
        input.recall_tol,
        static_cast<uint8_t>(input.early_termination),
        static_cast<uint8_t>(input.sample_queries),
        input.objective_k,
        input.ann_explore_fraction,
        input.min_not_pruned_pct,
        input.max_not_pruned_pct,
        input.adjustment_factor_for_partial_d,
        static_cast<uint8_t>(input.unrotate_centroids),
        static_cast<uint8_t>(input.verbose),
        static_cast<uint8_t>(input.angular),
        static_cast<uint8_t>(input.suppress_warnings),
        static_cast<uint8_t>(input.data_already_rotated),
        input.iters_mesoclustering,
        input.iters_fineclustering,
        input.iters_refinement,
    };
}

static SkmIterationStats to_c_stats(const skmeans::SuperKMeansIterationStats& stats) {
    return SkmIterationStats{
        stats.iteration,
        stats.objective,
        stats.shift,
        stats.split,
        stats.recall,
        stats.not_pruned_pct,
        stats.partial_d,
        static_cast<uint8_t>(stats.is_gemm_only),
    };
}

static SkmClusterBalanceStats to_c_cluster_balance_stats(
    const skmeans::ClusterBalanceStats& stats
) {
    return SkmClusterBalanceStats{
        stats.mean,
        stats.geometric_mean,
        stats.stdev,
        stats.cv,
        stats.min,
        stats.max,
    };
}

extern "C" {

void skm_string_free(char* value) {
    std::free(value);
}

void skm_buffer_free(void* value) {
    std::free(value);
}

SkmHierHandle* skm_hier_new(
    size_t n_clusters,
    size_t dimensionality,
    const SkmHierConfig* config,
    char** out_error
) {
    try {
        auto cpp_config = to_cpp_config(config);
        auto* handle = new SkmHierHandle{};
        handle->ptr = new HierarchicalKMeans(n_clusters, dimensionality, cpp_config);
        return handle;
    } catch (const std::exception& ex) {
        set_error(out_error, ex.what());
        return nullptr;
    } catch (...) {
        set_error(out_error, "unknown C++ exception");
        return nullptr;
    }
}

void skm_hier_free(SkmHierHandle* handle) {
    if (!handle) {
        return;
    }
    delete handle->ptr;
    delete handle;
}

int skm_hier_train(
    SkmHierHandle* handle,
    const float* data,
    size_t n,
    const float* queries,
    size_t n_queries,
    float** out_centroids,
    size_t* out_len
) {
    try {
        auto centroids = handle->ptr->Train(data, n, queries, n_queries);
        auto bytes = centroids.size() * sizeof(float);
        *out_centroids = static_cast<float*>(copy_to_heap(centroids.data(), bytes));
        *out_len = centroids.size();
        handle->last_error.clear();
        return 0;
    } catch (const std::exception& ex) {
        handle->last_error = ex.what();
        return 1;
    } catch (...) {
        handle->last_error = "unknown C++ exception";
        return 1;
    }
}

int skm_hier_assign(
    SkmHierHandle* handle,
    const float* vectors,
    const float* centroids,
    size_t n_vectors,
    size_t n_centroids,
    uint32_t** out_assignments,
    size_t* out_len
) {
    try {
        auto assignments = handle->ptr->Assign(vectors, centroids, n_vectors, n_centroids);
        auto bytes = assignments.size() * sizeof(uint32_t);
        *out_assignments = static_cast<uint32_t*>(copy_to_heap(assignments.data(), bytes));
        *out_len = assignments.size();
        handle->last_error.clear();
        return 0;
    } catch (const std::exception& ex) {
        handle->last_error = ex.what();
        return 1;
    } catch (...) {
        handle->last_error = "unknown C++ exception";
        return 1;
    }
}

int skm_hier_assign_training_points(
    SkmHierHandle* handle,
    const float* vectors,
    const float* centroids,
    size_t n_vectors,
    size_t n_centroids,
    uint32_t** out_assignments,
    size_t* out_len
) {
    try {
        auto assignments =
            handle->ptr->AssignTrainingPoints(vectors, centroids, n_vectors, n_centroids);
        auto bytes = assignments.size() * sizeof(uint32_t);
        *out_assignments = static_cast<uint32_t*>(copy_to_heap(assignments.data(), bytes));
        *out_len = assignments.size();
        handle->last_error.clear();
        return 0;
    } catch (const std::exception& ex) {
        handle->last_error = ex.what();
        return 1;
    } catch (...) {
        handle->last_error = "unknown C++ exception";
        return 1;
    }
}

const char* skm_hier_last_error(const SkmHierHandle* handle) {
    if (!handle) {
        return "null SkmHierHandle";
    }
    return handle->last_error.c_str();
}

size_t skm_hier_n_clusters(const SkmHierHandle* handle) {
    return handle->ptr->GetNClusters();
}

uint8_t skm_hier_is_trained(const SkmHierHandle* handle) {
    return static_cast<uint8_t>(handle->ptr->IsTrained());
}

float skm_hier_sampling_fraction(const SkmHierHandle* handle) {
    return handle->ptr->GetSamplingFraction();
}

void skm_hier_copy_config(const SkmHierHandle* handle, SkmHierConfig* out) {
    *out = to_c_config(handle->ptr->hierarchical_config);
}

const float* skm_hier_distances(const SkmHierHandle* handle) {
    return handle->ptr->GetDistancesPointer();
}

size_t skm_hier_mesostats_len(const SkmHierHandle* handle) {
    return handle->ptr->hierarchical_iteration_stats.mesoclustering_iteration_stats.size();
}

size_t skm_hier_finestats_len(const SkmHierHandle* handle) {
    return handle->ptr->hierarchical_iteration_stats.fineclustering_iteration_stats.size();
}

size_t skm_hier_refinestats_len(const SkmHierHandle* handle) {
    return handle->ptr->hierarchical_iteration_stats.refinement_iteration_stats.size();
}

void skm_hier_copy_mesostats(const SkmHierHandle* handle, SkmIterationStats* out) {
    const auto& stats = handle->ptr->hierarchical_iteration_stats.mesoclustering_iteration_stats;
    for (size_t i = 0; i < stats.size(); ++i) {
        out[i] = to_c_stats(stats[i]);
    }
}

void skm_hier_copy_finestats(const SkmHierHandle* handle, SkmIterationStats* out) {
    const auto& stats = handle->ptr->hierarchical_iteration_stats.fineclustering_iteration_stats;
    for (size_t i = 0; i < stats.size(); ++i) {
        out[i] = to_c_stats(stats[i]);
    }
}

void skm_hier_copy_refinestats(const SkmHierHandle* handle, SkmIterationStats* out) {
    const auto& stats = handle->ptr->hierarchical_iteration_stats.refinement_iteration_stats;
    for (size_t i = 0; i < stats.size(); ++i) {
        out[i] = to_c_stats(stats[i]);
    }
}

size_t skm_hier_get_n_mesoclusters(size_t n_clusters) {
    return HierarchicalKMeans::GetNMesoclusters(n_clusters);
}

size_t skm_hier_get_n_vectors_to_sample(
    const SkmHierHandle* handle,
    size_t n,
    size_t n_clusters
) {
    return handle->ptr->GetNVectorsToSample(n, n_clusters);
}

SkmClusterBalanceStats skm_hier_get_clusters_balance_stats(
    const uint32_t* assignments,
    size_t n_samples,
    size_t n_clusters
) {
    return to_c_cluster_balance_stats(
        HierarchicalKMeans::GetClustersBalanceStats(assignments, n_samples, n_clusters)
    );
}

}
