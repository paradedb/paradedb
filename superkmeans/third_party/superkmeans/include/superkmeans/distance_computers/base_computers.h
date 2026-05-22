#pragma once

#include "superkmeans/common.h"

#ifdef __ARM_NEON
#include "neon_computers.h"
#endif

#if defined(__AVX2__) && !defined(__AVX512F__)
#include "avx2_computers.h"
#endif

#ifdef __AVX512F__
#include "avx512_computers.h"
#endif

#if !defined(__ARM_NEON) && !defined(__AVX2__) && !defined(__AVX512F__)
#include "scalar_computers.h"
#endif

namespace skmeans {

template <DistanceFunction alpha, Quantization q>
class DistanceComputer {};

template <>
class DistanceComputer<DistanceFunction::l2, Quantization::f32> {
#if !defined(__ARM_NEON) && !defined(__AVX2__) && !defined(__AVX512F__)
    using computer = ScalarComputer<DistanceFunction::l2, Quantization::f32>;
#else
    using computer = SIMDComputer<DistanceFunction::l2, Quantization::f32>;
#endif

  public:
    constexpr static auto Horizontal = computer::Horizontal;
};

// template <>
// class DistanceComputer<DistanceFunction::l2, Quantization::u8> {
//     using computer = SIMDComputer<DistanceFunction::l2, Quantization::u8>;
//   public:
//     constexpr static auto Horizontal = computer::Horizontal;
// };

template <Quantization q>
class UtilsComputer {
#if !defined(__ARM_NEON) && !defined(__AVX2__) && !defined(__AVX512F__)
    using computer = ScalarUtilsComputer<q>;
#else
    using computer = SIMDUtilsComputer<q>;
#endif

  public:
    constexpr static auto FlipSign = computer::FlipSign;
    constexpr static auto InitPositionsArray = computer::InitPositionsArray;
};

} // namespace skmeans
