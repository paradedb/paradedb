#pragma once

#include <iostream>

#include "arm_neon.h"
#include "superkmeans/common.h"

namespace skmeans {

template <DistanceFunction alpha, Quantization q>
class SIMDComputer {};

template <>
class SIMDComputer<DistanceFunction::l2, Quantization::u8> {
  public:
    using distance_t = skmeans_distance_t<Quantization::u8>;
    using value_t = skmeans_value_t<Quantization::u8>;

    /**
     * @brief Computes the L2 distance between two uint8 vectors using NEON.
     * Taken from SimSimd library: https://github.com/ashvardanian/SimSIMD
     * @param vector1 Input vector 1
     * @param vector2 Input vector 2
     * @param num_dimensions Number of dimensions
     * @return L2 distance between the two vectors
     */
    static distance_t Horizontal(
        const value_t* SKM_RESTRICT vector1,
        const value_t* SKM_RESTRICT vector2,
        size_t num_dimensions
    ) {
        uint32x4_t sum_vec = vdupq_n_u32(0);
        size_t i = 0;
        for (; i + 16 <= num_dimensions; i += 16) {
            uint8x16_t a_vec = vld1q_u8(vector1 + i);
            uint8x16_t b_vec = vld1q_u8(vector2 + i);
            uint8x16_t d_vec = vabdq_u8(a_vec, b_vec);
            sum_vec = vdotq_u32(sum_vec, d_vec, d_vec);
        }
        distance_t distance = vaddvq_u32(sum_vec);
        for (; i < num_dimensions; ++i) {
            int n = (int) vector1[i] - vector2[i];
            distance += n * n;
        }
        return distance;
    };
};

template <>
class SIMDComputer<DistanceFunction::l2, Quantization::f32> {
  public:
    using distance_t = skmeans_distance_t<Quantization::f32>;
    using data_t = skmeans_value_t<Quantization::f32>;

    /**
     * @brief Computes the L2 distance between two float vectors using NEON.
     * Taken from SimSimd library: https://github.com/ashvardanian/SimSIMD
     * @param vector1 Input vector 1
     * @param vector2 Input vector 2
     * @param num_dimensions Number of dimensions
     * @return L2 distance between the two vectors
     */
    static distance_t Horizontal(
        const data_t* SKM_RESTRICT vector1,
        const data_t* SKM_RESTRICT vector2,
        size_t num_dimensions
    ) {
#if defined(__APPLE__)
        distance_t distance = 0.0f;
        SKM_VECTORIZE_LOOP
        for (size_t i = 0; i < num_dimensions; ++i) {
            float diff = vector1[i] - vector2[i];
            distance += diff * diff;
        }
        return distance;
#else
        float32x4_t sum_vec = vdupq_n_f32(0);
        size_t i = 0;
        for (; i + 4 <= num_dimensions; i += 4) {
            float32x4_t a_vec = vld1q_f32(vector1 + i);
            float32x4_t b_vec = vld1q_f32(vector2 + i);
            float32x4_t diff_vec = vsubq_f32(a_vec, b_vec);
            sum_vec = vfmaq_f32(sum_vec, diff_vec, diff_vec);
        }
        distance_t distance = vaddvq_f32(sum_vec);
        for (; i < num_dimensions; ++i) {
            float diff = vector1[i] - vector2[i];
            distance += diff * diff;
        }
        return distance;
#endif
    };
};

template <Quantization q>
class SIMDUtilsComputer {};

template <>
class SIMDUtilsComputer<Quantization::f32> {
  public:
    using data_t = skmeans_value_t<Quantization::f32>;

    /**
     * @brief Flip sign of floats based on a mask using NEON.
     * @param data Input vector (d elements)
     * @param out Output vector (can be same as data for in-place)
     * @param masks Bitmask array (0x80000000 to flip, 0 to keep)
     * @param d Number of dimensions
     */
    static void FlipSign(const data_t* data, data_t* out, const uint32_t* masks, size_t d) {
        size_t j = 0;
        for (; j + 4 <= d; j += 4) {
            float32x4_t vec = vld1q_f32(data + j);
            const uint32x4_t mask = vld1q_u32(masks + j);
            vec = vreinterpretq_f32_u32(veorq_u32(vreinterpretq_u32_f32(vec), mask));
            vst1q_f32(out + j, vec);
        }
        auto data_bits = reinterpret_cast<const uint32_t*>(data);
        auto out_bits = reinterpret_cast<uint32_t*>(out);
        for (; j < d; ++j) {
            out_bits[j] = data_bits[j] ^ masks[j];
        }
    }

    /**
     * @brief Initializes positions array with indices of non-pruned vectors using NEON.
     *
     * Optimized for cases where only ~2% of vectors pass the threshold test.
     * This version is only slightly faster than a scalar kernel
     *
     * @param n_vectors Number of vectors to process
     * @param n_vectors_not_pruned Output: count of vectors passing threshold (updated)
     * @param pruning_positions Output array of indices that passed (compacted)
     * @param pruning_threshold Threshold value for comparison
     * @param pruning_distances Input array of distances to compare
     */
    static void InitPositionsArray(
        size_t n_vectors,
        size_t& n_vectors_not_pruned,
        uint32_t* pruning_positions,
        data_t pruning_threshold,
        const data_t* pruning_distances
    ) {
        n_vectors_not_pruned = 0;
        size_t vector_idx = 0;
        constexpr size_t k_simd_width = 4;
        const size_t n_vectors_simd = (n_vectors / k_simd_width) * k_simd_width;
        float32x4_t threshold_vec = vdupq_n_f32(pruning_threshold);
        for (; vector_idx < n_vectors_simd; vector_idx += k_simd_width) {
            float32x4_t distances = vld1q_f32(pruning_distances + vector_idx);
            uint32x4_t cmp_result = vcltq_f32(distances, threshold_vec);
            uint32_t any_passed = vmaxvq_u32(cmp_result);
            if (SKM_UNLIKELY(any_passed)) {
                uint32_t mask[4];
                vst1q_u32(mask, cmp_result);
                for (size_t i = 0; i < k_simd_width; ++i) {
                    pruning_positions[n_vectors_not_pruned] = vector_idx + i;
                    n_vectors_not_pruned += (mask[i] != 0);
                }
            }
        }
        for (; vector_idx < n_vectors; ++vector_idx) {
            pruning_positions[n_vectors_not_pruned] = vector_idx;
            n_vectors_not_pruned += pruning_distances[vector_idx] < pruning_threshold;
        }
    }
};

} // namespace skmeans
