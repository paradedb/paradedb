#pragma once

#include "superkmeans/common.h"
#include <vector>

namespace skmeans {

/*
 * PDX index structure for IVF
 */
template <Quantization q>
class IndexPDXIVF {};

template <>
class IndexPDXIVF<Quantization::f32> {
  public:
    using CLUSTER_TYPE = Cluster<Quantization::f32>;

    uint32_t num_dimensions{};
    uint32_t num_clusters{};
    uint32_t num_horizontal_dimensions{};
    uint32_t num_vertical_dimensions{};
    std::vector<CLUSTER_TYPE> clusters;
};

template <>
class IndexPDXIVF<Quantization::u8> {
  public:
    using CLUSTER_TYPE = Cluster<Quantization::u8>;

    uint32_t num_dimensions{};
    uint32_t num_clusters{};
    uint32_t num_horizontal_dimensions{};
    uint32_t num_vertical_dimensions{};
    std::vector<Cluster<Quantization::u8>> clusters;

    float for_base{};
    float scale_factor{};
};

} // namespace skmeans
