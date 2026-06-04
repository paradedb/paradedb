#pragma once

#include <immintrin.h>

#include <cstdint>
#include <cstdio>

#include "superkmeans/common.h"
#include "superkmeans/distance_computers/scalar_computers.h"

namespace skmeans {

template <DistanceFunction alpha, Quantization q>
class SIMDComputer {};

template <>
class SIMDComputer<skmeans::DistanceFunction::l2, skmeans::Quantization::u8> {};

template <>
class SIMDComputer<skmeans::DistanceFunction::l2, skmeans::Quantization::f32> {
  public:
    using distance_t = skmeans_distance_t<skmeans::Quantization::f32>;
    using data_t = skmeans_value_t<skmeans::Quantization::f32>;
    using scalar_computer =
        ScalarComputer<skmeans::DistanceFunction::l2, skmeans::Quantization::f32>;

    /**
     * @brief Computes the L2 distance between two float vectors using AVX2.
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
        __m256 d2_vec = _mm256_setzero_ps();
        size_t i = 0;
        for (; i + 8 <= num_dimensions; i += 8) {
            __m256 a_vec = _mm256_loadu_ps(vector1 + i);
            __m256 b_vec = _mm256_loadu_ps(vector2 + i);
            __m256 d_vec = _mm256_sub_ps(a_vec, b_vec);
            d2_vec = _mm256_fmadd_ps(d_vec, d_vec, d2_vec);
        }

        // _simsimd_reduce_f32x8_haswell
        // Convert the lower and higher 128-bit lanes of the input vector to double precision
        __m128 low_f32 = _mm256_castps256_ps128(d2_vec);
        __m128 high_f32 = _mm256_extractf128_ps(d2_vec, 1);

        // Convert single-precision (float) vectors to double-precision (double) vectors
        __m256d low_f64 = _mm256_cvtps_pd(low_f32);
        __m256d high_f64 = _mm256_cvtps_pd(high_f32);

        // Perform the addition in double-precision
        __m256d sum = _mm256_add_pd(low_f64, high_f64);

        // Reduce the double-precision vector to a scalar
        // Horizontal add the first and second double-precision values, and third and fourth
        __m128d sum_low = _mm256_castpd256_pd128(sum);
        __m128d sum_high = _mm256_extractf128_pd(sum, 1);
        __m128d sum128 = _mm_add_pd(sum_low, sum_high);

        // Horizontal add again to accumulate all four values into one
        sum128 = _mm_hadd_pd(sum128, sum128);

        // Convert the final sum to a scalar double-precision value and return
        double d2 = _mm_cvtsd_f64(sum128);

        SKM_VECTORIZE_LOOP
        for (; i < num_dimensions; ++i) {
            float d = vector1[i] - vector2[i];
            d2 += d * d;
        }

        return static_cast<distance_t>(d2); // NOLINT(bugprone-narrowing-conversions)
    };
};

template <>
class SIMDComputer<skmeans::DistanceFunction::dp, skmeans::Quantization::f32> {
  public:
    using distance_t = skmeans_distance_t<skmeans::Quantization::f32>;
    using data_t = skmeans_value_t<skmeans::Quantization::f32>;

    /**
     * @brief Computes the Dot Product of two float vectors using AVX2.
     * Taken from: https://github.com/ashvardanian/SimSIMD
     * @param vector1 Input vector 1
     * @param vector2 Input vector 2
     * @param num_dimensions Number of dimensions
     * @return Dot Product between the two vectors
     */
    static distance_t Horizontal(
        const data_t* SKM_RESTRICT vector1,
        const data_t* SKM_RESTRICT vector2,
        size_t num_dimensions
    ) {
        __m256 d2_vec = _mm256_setzero_ps();
        size_t i = 0;
        for (; i + 8 <= num_dimensions; i += 8) {
            __m256 a_vec = _mm256_loadu_ps(vector1 + i);
            __m256 b_vec = _mm256_loadu_ps(vector2 + i);
            d2_vec = _mm256_fmadd_ps(a_vec, b_vec, d2_vec);
        }

        // _simsimd_reduce_f32x8_haswell
        // Convert the lower and higher 128-bit lanes of the input vector to double precision
        __m128 low_f32 = _mm256_castps256_ps128(d2_vec);
        __m128 high_f32 = _mm256_extractf128_ps(d2_vec, 1);

        // Convert single-precision (float) vectors to double-precision (double) vectors
        __m256d low_f64 = _mm256_cvtps_pd(low_f32);
        __m256d high_f64 = _mm256_cvtps_pd(high_f32);

        // Perform the addition in double-precision
        __m256d sum = _mm256_add_pd(low_f64, high_f64);

        // Reduce the double-precision vector to a scalar
        // Horizontal add the first and second double-precision values, and third and fourth
        __m128d sum_low = _mm256_castpd256_pd128(sum);
        __m128d sum_high = _mm256_extractf128_pd(sum, 1);
        __m128d sum128 = _mm_add_pd(sum_low, sum_high);

        // Horizontal add again to accumulate all four values into one
        sum128 = _mm_hadd_pd(sum128, sum128);

        // Convert the final sum to a scalar double-precision value and return
        double d2 = _mm_cvtsd_f64(sum128);

        for (; i < num_dimensions; ++i) {
            d2 += vector1[i] * vector2[i];
        }
        return static_cast<distance_t>(d2); // NOLINT(bugprone-narrowing-conversions)
    };
};

template <Quantization q>
class SIMDUtilsComputer {};

template <>
class SIMDUtilsComputer<skmeans::Quantization::f32> {
  public:
    using data_t = skmeans_value_t<skmeans::Quantization::f32>;

    /**
     * @brief Flip sign of floats based on a mask using AVX2.
     * @param data Input vector (d elements)
     * @param out Output vector (can be same as data for in-place)
     * @param masks Bitmask array (0x80000000 to flip, 0 to keep)
     * @param d Number of dimensions
     */
    static void FlipSign(const data_t* data, data_t* out, const uint32_t* masks, size_t d) {
        size_t j = 0;
        for (; j + 8 <= d; j += 8) {
            __m256 vec = _mm256_loadu_ps(data + j);
            __m256i mask = _mm256_loadu_si256(reinterpret_cast<const __m256i*>(masks + j));
            __m256i vec_i = _mm256_castps_si256(vec);
            vec_i = _mm256_xor_si256(vec_i, mask);
            _mm256_storeu_ps(out + j, _mm256_castsi256_ps(vec_i));
        }
        auto data_bits = reinterpret_cast<const uint32_t*>(data);
        auto out_bits = reinterpret_cast<uint32_t*>(out);
        for (; j < d; ++j) {
            out_bits[j] = data_bits[j] ^ masks[j];
        }
    }

    /**
     * @brief Initializes positions array with indices of non-pruned vectors using AVX2.
     *
     * Optimized for cases where only ~2% of vectors pass the threshold test.
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
        constexpr size_t k_simd_width = 8;
        const size_t n_vectors_simd = (n_vectors / k_simd_width) * k_simd_width;
        __m256 threshold_vec = _mm256_set1_ps(pruning_threshold);
        for (; vector_idx < n_vectors_simd; vector_idx += k_simd_width) {
            __m256 distances = _mm256_loadu_ps(pruning_distances + vector_idx);
            __m256 cmp_result = _mm256_cmp_ps(distances, threshold_vec, _CMP_LT_OQ);
            int mask = _mm256_movemask_ps(cmp_result);
            if (SKM_UNLIKELY(mask)) {
                for (size_t i = 0; i < k_simd_width; ++i) {
                    pruning_positions[n_vectors_not_pruned] = vector_idx + i;
                    n_vectors_not_pruned += (mask >> i) & 1;
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
