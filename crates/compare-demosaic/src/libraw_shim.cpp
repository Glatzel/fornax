#include <libraw/libraw.h>

#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <new>

namespace {

class ReferenceDemosaicer final : public LibRaw {
public:
  int linear(const std::uint16_t *mosaic, std::uint32_t width,
             std::uint32_t height, std::uint32_t filters,
             std::uint16_t *output) {
    if (width < 3 || height < 3) {
      return 1;
    }

    const std::size_t pixel_count =
        static_cast<std::size_t>(width) * static_cast<std::size_t>(height);
    if (pixel_count / width != height) {
      return 2;
    }

    auto *image = static_cast<ushort(*)[4]>(
        std::calloc(pixel_count, sizeof(ushort[4])));
    if (image == nullptr) {
      return 3;
    }

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

    pre_interpolate();
    lin_interpolate();

    for (std::size_t index = 0; index < pixel_count; ++index) {
      output[index * 3] = image[index][0];
      output[index * 3 + 1] = image[index][1];
      output[index * 3 + 2] = image[index][2];
    }
    return 0;
  }
};

} // namespace

extern "C" int compare_demosaic_libraw_linear(const std::uint16_t *mosaic,
                                               std::uint32_t width,
                                               std::uint32_t height,
                                               std::uint32_t filters,
                                               std::uint16_t *output) noexcept {
  if (mosaic == nullptr || output == nullptr) {
    return 4;
  }

  try {
    ReferenceDemosaicer reference;
    return reference.linear(mosaic, width, height, filters, output);
  } catch (const std::bad_alloc &) {
    return 5;
  } catch (...) {
    return 6;
  }
}
