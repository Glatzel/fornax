#include "shim.h"

#include <cstddef>
#include <cstdlib>
#include <limits>
#include <new>

compare_demosaic_status
ReferenceDemosaicer::prepare(const std::uint16_t *mosaic,
                             std::uint32_t width, std::uint32_t height,
                             std::uint32_t filters) {
  constexpr auto ushort_max = std::numeric_limits<ushort>::max();
  if (width < 3 || height < 3) {
    return compare_demosaic_invalid_dimensions;
  }
  if (width > ushort_max || height > ushort_max) {
    return compare_demosaic_dimension_overflow;
  }
  if (static_cast<std::size_t>(width) >
      std::numeric_limits<std::size_t>::max() / height) {
    return compare_demosaic_dimension_overflow;
  }

  const auto pixel_count =
      static_cast<std::size_t>(width) * static_cast<std::size_t>(height);
  auto *image = static_cast<ushort(*)[4]>(
      std::calloc(pixel_count, sizeof(ushort[4])));
  if (image == nullptr) {
    return compare_demosaic_allocation_failed;
  }

  // LibRaw::recycle() owns and frees imgdata.image.
  imgdata.image = image;
  imgdata.sizes.width = static_cast<ushort>(width);
  imgdata.sizes.height = static_cast<ushort>(height);
  imgdata.sizes.iwidth = static_cast<ushort>(width);
  imgdata.sizes.iheight = static_cast<ushort>(height);
  imgdata.idata.colors = 3;
  imgdata.idata.filters = filters;
  imgdata.params.four_color_rgb = 0;
  imgdata.params.half_size = 0;
  libraw_internal_data.internal_output_params.shrink = 0;

  for (std::uint32_t y = 0; y < height; ++y) {
    for (std::uint32_t x = 0; x < width; ++x) {
      const auto index = static_cast<std::size_t>(y) * width + x;
      const auto shift = (((y << 1U) & 14U) + (x & 1U)) * 2U;
      const auto channel = (filters >> shift) & 3U;
      image[index][channel] = mosaic[index];
    }
  }
  return compare_demosaic_ok;
}

void ReferenceDemosaicer::copy_rgb(std::uint16_t *output) const {
  const auto pixel_count =
      static_cast<std::size_t>(imgdata.sizes.width) * imgdata.sizes.height;
  for (std::size_t index = 0; index < pixel_count; ++index) {
    output[index * 3] = imgdata.image[index][0];
    output[index * 3 + 1] = imgdata.image[index][1];
    output[index * 3 + 2] = imgdata.image[index][2];
  }
}

extern "C" std::int32_t
compare_demosaic_libraw(std::uint32_t algorithm,
                        const std::uint16_t *mosaic, std::uint32_t width,
                        std::uint32_t height, std::uint32_t filters,
                        std::uint16_t *output) noexcept {
  if (mosaic == nullptr || output == nullptr) {
    return compare_demosaic_null_pointer;
  }

  try {
    ReferenceDemosaicer reference;
    const auto prepared = reference.prepare(mosaic, width, height, filters);
    if (prepared != compare_demosaic_ok) {
      return prepared;
    }

    compare_demosaic_status status = compare_demosaic_unknown_algorithm;
    switch (algorithm) {
    case 0:
      status = reference.run_linear();
      break;
    default:
      break;
    }
    if (status != compare_demosaic_ok) {
      return status;
    }

    reference.copy_rgb(output);
    return compare_demosaic_ok;
  } catch (const std::bad_alloc &) {
    return compare_demosaic_out_of_memory;
  } catch (...) {
    return compare_demosaic_exception;
  }
}
