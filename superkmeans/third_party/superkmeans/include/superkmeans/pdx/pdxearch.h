#pragma once

#include "superkmeans/distance_computers/base_computers.h"
#include "superkmeans/pdx/adsampling.h"
#include "superkmeans/pdx/pdx_ivf.h"
#include "superkmeans/pdx/utils.h"
#include <algorithm>
#include <cassert>

namespace skmeans {

/**
 * @brief PDXearch
 *
 * Implements the PDXearch algorithm for finding the nearest neighbor using the PDX
 * data layout combined with ADSampling-based pruning.
 * In this lightweight version, we optimize for top-1 search.
 * Reference: https://dl.acm.org/doi/abs/10.1145/3725333
 *
 * @tparam q Quantization type (f32 or u8)
 * @tparam Index
 * @tparam alpha Distance function (l2 or dp)
 */
template <
    Quantization q = Quantization::f32,
    class Index = IndexPDXIVF<q>,
    DistanceFunction alpha = DistanceFunction::l2>
class PDXearch {
  public:
    using DISTANCES_TYPE = skmeans_distance_t<q>;
    using DATA_TYPE = skmeans_value_t<q>;
    using INDEX_TYPE = Index;
    using CLUSTER_TYPE = Cluster<q>;
    using KNNCandidate_t = KNNCandidate<q>;
    using VectorComparator_t = VectorComparator<q>;
    using Pruner = ADSamplingPruner<q>;

    Pruner& pruner;
    INDEX_TYPE& pdx_data;

    /**
     * @brief Constructor
     *
     * @param data_index Reference to the PDX index containing the data
     * @param pruner Reference to the ADSampling pruner for threshold computation
     */
    PDXearch(INDEX_TYPE& data_index, Pruner& pruner) : pruner(pruner), pdx_data(data_index) {}

  protected:
    /**
     * @brief Retrieves the pruning threshold from the pruner.
     */
    template <Quantization Q = q>
    void GetPruningThreshold(
        KNNCandidate<Q>& best_candidate,
        skmeans_distance_t<Q>& pruning_threshold,
        uint32_t current_dimension_idx
    ) {
        pruning_threshold =
            pruner.template GetPruningThreshold<Q>(best_candidate, current_dimension_idx);
    }

    /**
     * @brief Updates the positions array to keep only non-pruned vectors.
     */
    template <Quantization Q = q>
    void EvaluatePruningPredicateOnPositionsArray(
        size_t n_vectors,
        size_t& n_vectors_not_pruned,
        uint32_t* pruning_positions,
        skmeans_distance_t<Q> pruning_threshold,
        skmeans_distance_t<Q>* pruning_distances
    ) {
        n_vectors_not_pruned = 0;
        for (size_t vector_idx = 0; vector_idx < n_vectors; ++vector_idx) {
            pruning_positions[n_vectors_not_pruned] = pruning_positions[vector_idx];
            n_vectors_not_pruned +=
                pruning_distances[pruning_positions[vector_idx]] < pruning_threshold;
        }
    }

    /**
     * @brief Initializes the positions array with indices of non-pruned vectors.
     */
    template <Quantization Q = q>
    void InitPositionsArray(
        size_t n_vectors,
        size_t& n_vectors_not_pruned,
        uint32_t* pruning_positions,
        skmeans_distance_t<Q> pruning_threshold,
        skmeans_distance_t<Q>* pruning_distances
    ) {
        UtilsComputer<Q>::InitPositionsArray(
            n_vectors, n_vectors_not_pruned, pruning_positions, pruning_threshold, pruning_distances
        );
    }

    /**
     * @brief Prune phase: iteratively eliminates vectors using remaining dimensions.
     *
     * @tparam Q Quantization type
     * @param query Query vector
     * @param data PDX-formatted cluster data
     * @param n_vectors Number of vectors in the cluster
     * @param pruning_positions Array of candidate positions (compacted in place)
     * @param pruning_distances Array of partial distances (updated in place)
     * @param pruning_threshold Current pruning threshold (updated)
     * @param best_candidate Current best candidate
     * @param n_vectors_not_pruned Number of remaining candidates (updated)
     * @param current_dimension_idx Number of dimensions processed (updated)
     * @param vector_indices Mapping from local to global vector indices
     * @param prev_top_1 Previous best candidate index (for early exit)
     * @param aux_vertical_dimensions_in_horizontal_layout Auxiliary horizontal data for
     * vertical dimensions
     * @param initial_not_pruned_out Output for starting non-pruned count
     */
    template <Quantization Q = q>
    void Prune(
        const skmeans_value_t<Q>* SKM_RESTRICT query,
        const skmeans_value_t<Q>* SKM_RESTRICT data,
        const size_t n_vectors,
        uint32_t* pruning_positions,
        skmeans_distance_t<Q>* pruning_distances,
        skmeans_distance_t<Q>& pruning_threshold,
        KNNCandidate<Q>& best_candidate,
        size_t& n_vectors_not_pruned,
        uint32_t& current_dimension_idx,
        const uint32_t* vector_indices,
        const uint32_t prev_top_1,
        const skmeans_value_t<Q>* SKM_RESTRICT aux_vertical_dimensions_in_horizontal_layout,
        size_t& initial_not_pruned_out
    ) {
        GetPruningThreshold<Q>(best_candidate, pruning_threshold, current_dimension_idx);
        InitPositionsArray<Q>(
            n_vectors, n_vectors_not_pruned, pruning_positions, pruning_threshold, pruning_distances
        );
        // Record the initial n_vectors_not_pruned
        initial_not_pruned_out = n_vectors_not_pruned;
        // Early exit if the only remaining point is the one that was initially the best candidate
        if (n_vectors_not_pruned == 1 && vector_indices[pruning_positions[0]] == prev_top_1) {
            n_vectors_not_pruned = 0;
            return;
        }
        size_t cur_n_vectors_not_pruned = 0;
        size_t current_vertical_dimension = current_dimension_idx;
        size_t current_horizontal_dimension = 0;

        // Go through the horizontal dimensions 64 at a time
        while (pdx_data.num_horizontal_dimensions && n_vectors_not_pruned &&
               current_horizontal_dimension < pdx_data.num_horizontal_dimensions) {
            cur_n_vectors_not_pruned = n_vectors_not_pruned;
            size_t offset_data = (pdx_data.num_vertical_dimensions * n_vectors) +
                                 (current_horizontal_dimension * n_vectors);
            for (size_t vector_idx = 0; vector_idx < n_vectors_not_pruned; vector_idx++) {
                size_t v_idx = pruning_positions[vector_idx];
                size_t data_pos = offset_data + (v_idx * H_DIM_SIZE);
                SKM_PREFETCH(data + data_pos, 0, 2);
            }
            size_t offset_query = pdx_data.num_vertical_dimensions + current_horizontal_dimension;
            for (size_t vector_idx = 0; vector_idx < n_vectors_not_pruned; vector_idx++) {
                size_t v_idx = pruning_positions[vector_idx];
                size_t data_pos = offset_data + (v_idx * H_DIM_SIZE);
                pruning_distances[v_idx] += DistanceComputer<alpha, Q>::Horizontal(
                    query + offset_query, data + data_pos, H_DIM_SIZE
                );
            }
            current_horizontal_dimension += H_DIM_SIZE;
            current_dimension_idx += H_DIM_SIZE;
            GetPruningThreshold<Q>(best_candidate, pruning_threshold, current_dimension_idx);
            assert(
                current_dimension_idx == current_vertical_dimension + current_horizontal_dimension
            );
            EvaluatePruningPredicateOnPositionsArray<Q>(
                cur_n_vectors_not_pruned,
                n_vectors_not_pruned,
                pruning_positions,
                pruning_threshold,
                pruning_distances
            );
        }

        // Go through all the remaining vertical dimensions stored in the horizontal layout
        if (n_vectors_not_pruned && current_vertical_dimension < pdx_data.num_vertical_dimensions) {
            cur_n_vectors_not_pruned = n_vectors_not_pruned;
            size_t dimensions_left = pdx_data.num_vertical_dimensions - current_vertical_dimension;
            size_t offset_query = current_vertical_dimension;
            for (size_t vector_idx = 0; vector_idx < n_vectors_not_pruned; vector_idx++) {
                size_t v_idx = pruning_positions[vector_idx];
                auto data_pos = aux_vertical_dimensions_in_horizontal_layout +
                                (v_idx * pdx_data.num_vertical_dimensions) +
                                current_vertical_dimension;
                SKM_PREFETCH(data_pos, 0, 1);
            }
            for (size_t vector_idx = 0; vector_idx < n_vectors_not_pruned; vector_idx++) {
                size_t v_idx = pruning_positions[vector_idx];
                auto data_pos = aux_vertical_dimensions_in_horizontal_layout +
                                (v_idx * pdx_data.num_vertical_dimensions) +
                                current_vertical_dimension;
                pruning_distances[v_idx] += DistanceComputer<alpha, Q>::Horizontal(
                    query + offset_query, data_pos, dimensions_left
                );
            }
            current_dimension_idx = pdx_data.num_dimensions;
            current_vertical_dimension = pdx_data.num_vertical_dimensions;
            assert(
                current_dimension_idx == current_vertical_dimension + current_horizontal_dimension
            );
            GetPruningThreshold<Q>(best_candidate, pruning_threshold, current_dimension_idx);
            EvaluatePruningPredicateOnPositionsArray<Q>(
                cur_n_vectors_not_pruned,
                n_vectors_not_pruned,
                pruning_positions,
                pruning_threshold,
                pruning_distances
            );
        }
    }

    /**
     * @brief Updates the best candidate from remaining non-pruned vectors.
     */
    template <Quantization Q = q>
    void SetBestCandidate(
        const uint32_t* vector_indices,
        size_t n_vectors,
        const uint32_t* pruning_positions,
        const skmeans_distance_t<Q>* pruning_distances,
        KNNCandidate<Q>& best_candidate
    ) {
        for (size_t position_idx = 0; position_idx < n_vectors; ++position_idx) {
            size_t index = pruning_positions[position_idx];
            skmeans_distance_t<Q> current_distance = pruning_distances[index];
            if (current_distance < best_candidate.distance) {
                best_candidate.distance = current_distance;
                best_candidate.index = vector_indices[index];
            }
        }
    }

    /**
     * @brief Converts distances back to original domain (for u8 quantization).
     */
    void BuildResultSet(KNNCandidate_t& best_candidate) {
        // We return distances in the original domain
        if constexpr (q == Quantization::u8) {
            float inverse_scale_factor = 1.0f / pdx_data.scale_factor;
            inverse_scale_factor = inverse_scale_factor * inverse_scale_factor;
            best_candidate.distance = best_candidate.distance * inverse_scale_factor;
        }
    }

  public:
    /**
     * @brief Finds the top-1 neighbor using partial distances from GEMM.
     *
     * @param query Query vector (in rotated space)
     * @param prev_pruning_threshold Initial distance threshold
     * @param prev_top_1 Index of previous best candidate
     * @param partial_pruning_distances Pre-computed partial distances (updated in place)
     * @param computed_distance_until Number of dimensions already computed
     * @param start_cluster First cluster index to search
     * @param end_cluster One past the last cluster index to search
     * @param initial_not_pruned_accum Accumulator for not-pruned statistics
     * @return Best candidate found (index and distance)
     */
    KNNCandidate_t Top1PartialSearchWithThresholdAndPartialDistances(
        const float* SKM_RESTRICT query,
        const float prev_pruning_threshold,
        const uint32_t prev_top_1,
        DISTANCES_TYPE* partial_pruning_distances,
        const uint32_t computed_distance_until,
        const size_t start_cluster,
        const size_t end_cluster,
        size_t& initial_not_pruned_accum
    ) {
        alignas(64) thread_local uint32_t pruning_positions[VECTOR_CHUNK_SIZE];
        DISTANCES_TYPE pruning_threshold = std::numeric_limits<DISTANCES_TYPE>::max();
        size_t n_vectors_not_pruned = 0;
        uint32_t current_dimension_idx = computed_distance_until;
        size_t current_cluster = 0;

        // Setup previous top1
        pruning_threshold = prev_pruning_threshold;
        auto top_embedding = KNNCandidate<q>{};
        top_embedding.index = prev_top_1;
        top_embedding.distance = prev_pruning_threshold;

        // Pruning core
        size_t data_offset = 0;
        for (size_t cluster_idx = start_cluster; cluster_idx < end_cluster; ++cluster_idx) {
            current_dimension_idx = computed_distance_until;

            auto pruning_distances = partial_pruning_distances + data_offset;
            current_cluster = cluster_idx;
            CLUSTER_TYPE& cluster = pdx_data.clusters[current_cluster];
            data_offset += cluster.num_embeddings;
            size_t initial_not_pruned = 0;
            Prune(
                query,
                cluster.data,
                cluster.num_embeddings,
                pruning_positions,
                pruning_distances,
                pruning_threshold,
                top_embedding,
                n_vectors_not_pruned,
                current_dimension_idx,
                cluster.indices,
                prev_top_1,
                cluster.aux_vertical_dimensions_in_horizontal_layout,
                initial_not_pruned
            );
            // Accumulate the initial not-pruned count for this cluster
            initial_not_pruned_accum += initial_not_pruned;
            if (n_vectors_not_pruned) {
                SetBestCandidate(
                    cluster.indices,
                    n_vectors_not_pruned,
                    pruning_positions,
                    pruning_distances,
                    top_embedding
                );
            }
        }
        BuildResultSet(top_embedding);
        return top_embedding;
    }
};

} // namespace skmeans
