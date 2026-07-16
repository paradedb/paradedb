#pragma once

#include <cinttypes>
#include <cstdint>
#include <cstdio>

extern "C" {
int sgemm_(
    const char* transa,
    const char* transb,
    int* m,
    int* n,
    int* k,
    const float* alpha,
    const float* a,
    int* lda,
    const float* b,
    int* ldb,
    float* beta,
    float* c,
    int* ldc
);
}

#define SKMEANS_ENSURE_POSITIVE(x)                                                                 \
    if ((x) <= 0) {                                                                                \
        throw std::invalid_argument("Value must be positive: " #x);                                \
    }

#ifndef SKM_RESTRICT
#if defined(__GNUC__) || defined(__clang__)
#define SKM_RESTRICT __restrict__
#elif defined(_MSC_VER)
#define SKM_RESTRICT __restrict
#elif defined(__INTEL_COMPILER)
#define SKM_RESTRICT __restrict__
#else
#define SKM_RESTRICT
#endif
#endif

#ifndef SKM_ALWAYS_INLINE
#if __has_cpp_attribute(gnu::always_inline)
#define SKM_ALWAYS_INLINE [[gnu::always_inline]]
#elif defined(__GNUC__) || defined(__clang__)
#define SKM_ALWAYS_INLINE __attribute__((always_inline))
#elif defined(_MSC_VER)
#define SKM_ALWAYS_INLINE __forceinline
#else
#define SKM_ALWAYS_INLINE
#endif
#endif

#ifndef SKM_NO_INLINE
#define SKM_NO_INLINE __attribute__((noinline))
#endif

#if defined(__GNUC__) || defined(__clang__)
#define SKM_LIKELY(x) __builtin_expect(!!(x), 1)
#define SKM_UNLIKELY(x) __builtin_expect(!!(x), 0)
#else
#define SKM_LIKELY(x) (x)
#define SKM_UNLIKELY(x) (x)
#endif

#if defined(__GNUC__) || defined(__clang__)
#define SKM_PREFETCH(addr, rw, locality) __builtin_prefetch((addr), (rw), (locality))
#elif defined(_MSC_VER)
#include <xmmintrin.h>
#define SKM_PREFETCH(addr, rw, locality)                                                           \
    _mm_prefetch(reinterpret_cast<const char*>(addr), _MM_HINT_T0)
#else
#define SKM_PREFETCH(addr, rw, locality) ((void) 0)
#endif

// Cross-compiler vectorization hint for loops.
// Clang: #pragma clang loop vectorize(enable)
// GCC:   #pragma GCC ivdep (asserts no loop-carried dependencies, enabling vectorization)
#if defined(__clang__)
#define SKM_VECTORIZE_LOOP _Pragma("clang loop vectorize(enable)")
#elif defined(__GNUC__)
#define SKM_VECTORIZE_LOOP _Pragma("GCC ivdep")
#else
#define SKM_VECTORIZE_LOOP
#endif

namespace skmeans {

static inline constexpr float PROPORTION_HORIZONTAL_DIM = 0.75;
static inline constexpr size_t D_THRESHOLD_FOR_DCT_ROTATION = 512;
static inline constexpr size_t H_DIM_SIZE = 64;

static inline constexpr uint32_t MIN_PARTIAL_D = 16;

// Thresholds below which GEMM-only (no pruning) is used
static inline constexpr size_t DIMENSION_THRESHOLD_FOR_PRUNING = 128;
static inline constexpr size_t N_CLUSTERS_THRESHOLD_FOR_PRUNING = 256;

#if defined(__APPLE__)
// AMX (used with Apple Accelerate) benefits from larger batch sizes
static inline constexpr size_t X_BATCH_SIZE = 40960;
static inline constexpr size_t Y_BATCH_SIZE = 2048;
static inline constexpr size_t MINI_BATCH_SIZE = 256;
#else
static inline constexpr size_t X_BATCH_SIZE = 4096;
static inline constexpr size_t Y_BATCH_SIZE = 1024;
#endif

static inline constexpr size_t VECTOR_CHUNK_SIZE = Y_BATCH_SIZE;

static inline constexpr size_t RECALL_CONVERGENCE_PATIENCE = 2;
static inline constexpr float CENTROID_PERTURBATION_EPS = 1.0f / 1024.0f;
// Epsilon parameter of ADSampling (Reference: https://dl.acm.org/doi/abs/10.1145/3589282)
static inline constexpr float PRUNER_INITIAL_THRESHOLD = 1.5f;
static inline constexpr float HIERARCHICAL_PRUNER_INITIAL_THRESHOLD = 1.1f;

// Global thread count for OpenMP parallel regions
// This is set by SuperKMeans constructor. Not ideal but needed for
// external functions (adsampling, batch_computers) that can't access class members.
inline uint32_t g_n_threads = 1;

template <class T, T val = 8>
static constexpr uint32_t AlignValue(T n) {
    return ((n + (val - 1)) / val) * val;
}

enum class DistanceFunction : uint8_t { l2, dp };

enum class Quantization : uint8_t { f32, u8, f16, bf16 };

template <Quantization q>
struct DistanceType {
    using type = uint32_t;
};
template <>
struct DistanceType<Quantization::f32> {
    using type = float;
};
template <Quantization q>
using skmeans_distance_t = typename DistanceType<q>::type;

template <Quantization q>
struct DataType {
    using type = uint8_t;
};
template <>
struct DataType<Quantization::f32> {
    using type = float;
};
template <Quantization q>
using skmeans_value_t = typename DataType<q>::type;

template <Quantization q>
struct CentroidDataType {
    using type = float;
};
template <>
struct CentroidDataType<Quantization::f32> {
    using type = float;
};
template <Quantization q>
using skmeans_centroid_value_t = typename CentroidDataType<q>::type;

template <Quantization q>
struct KNNCandidate {
    uint32_t index;
    float distance;
};

template <Quantization q>
struct VectorComparator {
    bool operator()(const KNNCandidate<q>& a, const KNNCandidate<q>& b) {
        return a.distance < b.distance;
    }
};

template <Quantization q>
struct Cluster {
    uint32_t num_embeddings{};
    uint32_t* indices = nullptr;
    skmeans_value_t<Quantization::u8>* data = nullptr;
    skmeans_value_t<Quantization::u8>* aux_vertical_dimensions_in_horizontal_layout =
        nullptr; // Contains the vertical dimensions minus partial_d in a horizontal layout, aka the
                 // ones not visited by GEMM
};

template <>
struct Cluster<Quantization::f32> {
    uint32_t num_embeddings{};
    uint32_t* indices = nullptr;
    skmeans_value_t<Quantization::f32>* data = nullptr;
    skmeans_value_t<Quantization::f32>* aux_vertical_dimensions_in_horizontal_layout =
        nullptr; // Contains the vertical dimensions minus partial_d in a horizontal layout, aka the
                 // ones not visited by GEMM
};

} // namespace skmeans
