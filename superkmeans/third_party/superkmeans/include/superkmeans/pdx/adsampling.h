#pragma once

#include "superkmeans/distance_computers/base_computers.h"
#include "superkmeans/pdx/utils.h"
#include <omp.h>
#include <random>

#include <Eigen/Dense>
#ifdef HAS_FFTW
#include <fftw3.h>
#endif

namespace skmeans {

/**
 * @brief ADSampling pruner
 *
 * Implements Adaptive Dimension Sampling (ADSampling) which enables early termination
 * during distance computations by predicting whether a candidate can be pruned based
 * on partial distance calculations. Uses a random rotation matrix to ensure dimensions
 * contribute equally to the distance.
 * Reference: https://dl.acm.org/doi/abs/10.1145/3589282
 *
 * For high-dimensional data (>= D_THRESHOLD_FOR_DCT_ROTATION), uses DCT-based rotation
 * which is more efficient than full matrix multiplication.
 *
 * @tparam q Quantization type (f32 or u8)
 */
template <Quantization q = Quantization::f32>
class ADSamplingPruner {
    using DISTANCES_TYPE = skmeans_distance_t<q>;
    using value_t = skmeans_value_t<q>;
    using KNNCandidate_t = KNNCandidate<q>;
    using VectorComparator_t = VectorComparator<q>;
    using MatrixR = Eigen::Matrix<float, Eigen::Dynamic, Eigen::Dynamic, Eigen::RowMajor>;

  public:
    uint32_t num_dimensions;
    std::vector<float> ratios; // Precomputed pruning threshold ratios

    /**
     * @brief Constructor
     *
     * @param num_dimensions_
     * @param epsilon0 Pruning threshold parameter (higher = more aggressive pruning, less accuracy)
     * @param seed Random seed for reproducible rotation matrix generation
     */
    ADSamplingPruner(uint32_t num_dimensions_, float epsilon0, uint32_t seed = 42)
        : num_dimensions(num_dimensions_), epsilon0(epsilon0) {
        InitializeRatios();
        std::mt19937 gen(seed); // NOLINT(bugprone-narrowing-conversions)
        bool matrix_created = false;
#ifdef HAS_FFTW
#ifdef __AVX2__
        // x86 machines don't behave well with FFTW with non-power-of-2 dimensions
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION && IsPowerOf2(num_dimensions)) {
#else
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION) {
#endif
            fftwf_init_threads();
            matrix.resize(1, num_dimensions);
            std::uniform_int_distribution<int> dist(0, 1);
            for (size_t i = 0; i < num_dimensions; ++i) {
                matrix(i) = dist(gen) ? 1.0f : -1.0f;
            }
            flip_masks.resize(num_dimensions);
            for (size_t i = 0; i < num_dimensions; ++i) {
                // Use matrix(i) which has the random +1/-1 values, not flip_masks[i] which is
                // uninitialized
                flip_masks[i] = (matrix(i) < 0.0f ? 0x80000000u : 0u);
            }
            matrix_created = true;
        }
#endif
        if (!matrix_created) {
            matrix.resize(num_dimensions, num_dimensions);
            std::normal_distribution<float> dist(0.0f, 1.0f);
            for (uint32_t i = 0; i < num_dimensions; ++i) {
                for (uint32_t j = 0; j < num_dimensions; ++j) {
                    matrix(i, j) = dist(gen);
                }
            }
            const Eigen::HouseholderQR<MatrixR> qr(matrix);
            matrix = qr.householderQ() * MatrixR::Identity(num_dimensions, num_dimensions);
        }
    }

    /**
     * @brief Pre-computes pruning threshold ratios for all dimension indices.
     *
     * Called during construction and when epsilon0 changes.
     */
    void InitializeRatios() {
        // + 1 to be able to map n_dims to 1.0f, avoiding a branch in GetPruningThreshold
        ratios.resize(num_dimensions + 1);
        for (size_t i = 0; i < num_dimensions + 1; ++i) {
            ratios[i] = GetRatio(i);
        }
    }

    /**
     * @brief Updates the pruning threshold parameter and recalculates ratios.
     * @param eps0 New epsilon0 value
     */
    void SetEpsilon0(float eps0) {
        epsilon0 = eps0;
        InitializeRatios();
    }

    /**
     * @brief Computes the pruning threshold for a given number of visited dimensions.
     *
     * @tparam Q Quantization type
     * @param best_candidate Current best candidate (threshold)
     * @param current_dimension_idx Number of dimensions visited so far
     * @return Pruning threshold - vectors with partial distance above this can be pruned
     */
    template <Quantization Q = q>
    skmeans_distance_t<Q> GetPruningThreshold(
        const KNNCandidate<Q>& best_candidate,
        const uint32_t current_dimension_idx
    ) {
        return best_candidate.distance * ratios[current_dimension_idx];
    }

    /**
     * @brief Applies sign flipping for DCT-based rotation (FFTW path).
     *
     * @param data Input vectors (row-major, n × num_dimensions)
     * @param out Output vectors (row-major, n × num_dimensions)
     * @param n Number of vectors
     */
    void FlipSign(const value_t* data, value_t* out, const size_t n) {
#pragma omp parallel for num_threads(g_n_threads)
        for (size_t i = 0; i < n; ++i) {
            const size_t offset = i * num_dimensions;
            UtilsComputer<q>::FlipSign(
                data + offset, out + offset, flip_masks.data(), num_dimensions
            );
        }
    }

    /**
     * @brief Rotates vectors
     *
     * Transforms vectors to a rotated space where dimensions contribute more equally
     * to the total distance, enabling effective early termination.
     *
     * For DCT path: applies sign flipping followed by DCT-II transform.
     * For matrix path: computes out = vectors * matrix^T.
     *
     * @param vectors Input vectors (row-major, n × num_dimensions)
     * @param out_buffer Output buffer for rotated vectors (n × num_dimensions)
     * @param n Number of vectors to rotate
     */
    void Rotate(
        const float* SKM_RESTRICT vectors,
        float* SKM_RESTRICT out_buffer,
        const uint32_t n
    ) {
        Eigen::Map<MatrixR> out(out_buffer, n, num_dimensions);
#ifdef HAS_FFTW
#ifdef __AVX2__
        // x86 machines don't behave well with FFTW with non-power-of-2 dimensions
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION && IsPowerOf2(num_dimensions)) {
#else
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION) {
#endif
            FlipSign(vectors, out_buffer, n);
            int n0 = static_cast<int>(num_dimensions);
            int howmany = static_cast<int>(n);
            fftw_r2r_kind kind[1] = {FFTW_REDFT10};
            auto flag = FFTW_MEASURE;
            if (IsPowerOf2(num_dimensions)) {
                flag = FFTW_ESTIMATE;
            }
            fftwf_plan plan;
            fftwf_plan_with_nthreads(static_cast<int>(g_n_threads));
            plan = fftwf_plan_many_r2r(
                1, &n0, howmany, out.data(), NULL, 1, n0, out.data(), NULL, 1, n0, kind, flag
            );
            fftwf_execute(plan);
            fftwf_destroy_plan(plan);
            const float s0 = std::sqrt(1.0f / (4.0f * num_dimensions));
            const float s = std::sqrt(1.0f / (2.0f * num_dimensions));
            out.col(0) *= s0;
            out.rightCols(num_dimensions - 1) *= s;
            return;
        }
#endif
        const char trans_a = 'T';
        const char trans_b = 'N';
        int m = static_cast<int>(num_dimensions);
        int n_blas = static_cast<int>(n);
        int k = static_cast<int>(num_dimensions);
        float alpha = 1.0f;
        float beta = 0.0f;
        int lda = static_cast<int>(num_dimensions);
        int ldb = static_cast<int>(num_dimensions);
        int ldc = static_cast<int>(num_dimensions);
        sgemm_(
            &trans_a,
            &trans_b,
            &m,
            &n_blas,
            &k,
            &alpha,
            matrix.data(),
            &lda,
            vectors,
            &ldb,
            &beta,
            out_buffer,
            &ldc
        );
    }

    /**
     * @brief Unrotate vectors (inverse of Rotate).
     * For orthonormal matrix Q: Rotate does out = vectors * Q^T, so Unrotate does out = vectors * Q
     * For DCT: applies inverse scaling, then inverse DCT (DCT-III), then FlipSign
     */
    void Unrotate(
        const float* SKM_RESTRICT rotated_vectors,
        float* SKM_RESTRICT out_buffer,
        const uint32_t n
    ) {
        Eigen::Map<MatrixR> out(out_buffer, n, num_dimensions);
#ifdef HAS_FFTW
#ifdef __AVX2__
        // x86 machines don't behave well with FFTW with non-power-of-2 dimensions
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION && IsPowerOf2(num_dimensions)) {
#else
        if (num_dimensions >= D_THRESHOLD_FOR_DCT_ROTATION) {
#endif
            // Copy input to output buffer for in-place transform
            std::memcpy(
                out_buffer, rotated_vectors, static_cast<size_t>(n) * num_dimensions * sizeof(float)
            );

            // Undo scaling (inverse of forward scaling)
            const float inv_s0 = std::sqrt(4.0f * num_dimensions);
            const float inv_s = std::sqrt(2.0f * num_dimensions);
            out.col(0) *= inv_s0;
            out.rightCols(num_dimensions - 1) *= inv_s;

            // Apply inverse DCT (DCT-III = FFTW_REDFT01)
            fftwf_init_threads();
            fftwf_plan_with_nthreads(static_cast<int>(g_n_threads));
            int n0 = static_cast<int>(num_dimensions);
            int howmany = static_cast<int>(n);
            fftw_r2r_kind kind[1] = {FFTW_REDFT01}; // DCT-III (inverse of DCT-II)
            auto flag = FFTW_MEASURE;
            if (IsPowerOf2(num_dimensions)) {
                flag = FFTW_ESTIMATE;
            }
            fftwf_plan plan;
            fftwf_plan_with_nthreads(static_cast<int>(g_n_threads));
            plan = fftwf_plan_many_r2r(
                1, &n0, howmany, out.data(), NULL, 1, n0, out.data(), NULL, 1, n0, kind, flag
            );
            fftwf_execute(plan);
            fftwf_destroy_plan(plan);

            // FFTW's DCT-III needs normalization by 1/(2*n)
            out *= (1.0f / (2.0f * num_dimensions));

            // Undo FlipSign (FlipSign is its own inverse)
            FlipSign(out_buffer, out_buffer, n);
            return;
        }
#endif
        // For orthonormal matrix: Q^{-1} = Q^T, and Rotate does v * Q^T, so Unrotate does v * Q
        const char trans_a = 'N';
        const char trans_b = 'N';
        int m = static_cast<int>(num_dimensions);
        int n_blas = static_cast<int>(n);
        int k = static_cast<int>(num_dimensions);
        float alpha = 1.0f;
        float beta = 0.0f;
        int lda = static_cast<int>(num_dimensions);
        int ldb = static_cast<int>(num_dimensions);
        int ldc = static_cast<int>(num_dimensions);
        sgemm_(
            &trans_a,
            &trans_b,
            &m,
            &n_blas,
            &k,
            &alpha,
            matrix.data(),
            &lda,
            rotated_vectors,
            &ldb,
            &beta,
            out_buffer,
            &ldc
        );
    }

  private:
    float epsilon0 = 1.5f;            // Pruning aggressiveness parameter
    MatrixR matrix;                   // Rotation matrix (or sign vector for DCT)
    std::vector<uint32_t> flip_masks; // Sign flip masks for DCT-based rotation

    /**
     * @brief Computes the pruning ratio for a given number of visited dimensions.
     *
     * Based on the ADSampling paper, the ratio accounts for the expected contribution
     * of remaining dimensions to the total distance.
     *
     * @param visited_dimensions Number of dimensions visited so far
     * @return Ratio to multiply with best distance to get pruning threshold
     */
    float GetRatio(const size_t visited_dimensions) {
        if (visited_dimensions == 0) {
            return 1.0;
        }
        if (visited_dimensions == num_dimensions) {
            return 1.0;
        }
        return static_cast<float>(
            static_cast<double>(visited_dimensions) / static_cast<double>(num_dimensions) *
            (1.0 + epsilon0 / std::sqrt(static_cast<double>(visited_dimensions))) *
            (1.0 + epsilon0 / std::sqrt(static_cast<double>(visited_dimensions)))
        );
    }
};

} // namespace skmeans
