#include "../shim.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstring>
#include <new>
#include <vector>

using gboolean = int;
using dt_aligned_pixel_t = float[4];
using std::size_t;

#define DT_PIXEL_SIMD_CHANNELS 4
#define DT_OMP_DECLARE_SIMD(...)
#define DT_OMP_FOR(...)
#define for_each_channel(_var, ...)                                            \
  for (size_t _var = 0; _var < DT_PIXEL_SIMD_CHANNELS; ++_var)
#define dt_omp_nontemporal(pointer)
#define FC(row, col, filters)                                                  \
  ((filters >> (((((row) << 1) & 14U) + ((col) & 1U)) * 2U)) & 3U)

// PPG is called with the edge-aware median disabled, so this dependency is
// unreachable but still required while compiling darktable's source.
static void pre_median(float *, const float *, int, int, std::uint32_t, int,
                       float) {}
static float *dt_alloc_align_float(size_t) { return nullptr; }
static void dt_free_align(float *) {}

#include "external/darktable/src/iop/demosaicing/ppg.c"

std::int32_t run_darktable_ppg(const std::uint16_t *mosaic, std::uint32_t width,
                               std::uint32_t height, std::uint32_t filters,
                               std::uint16_t *output) noexcept {
  const auto pixel_count = static_cast<size_t>(width) * height;
  try {
    // darktable normalizes both green sites to code 1 before demosaicing.
    // LibRaw-style masks use code 3 for the second green site.
    for (std::uint32_t shift = 0; shift < 32; shift += 2) {
      if (((filters >> shift) & 3U) == 3U) {
        filters &= ~(2U << shift);
      }
    }
    std::vector<float> input(pixel_count);
    std::vector<float> rgb(pixel_count * 4);
    for (size_t i = 0; i < pixel_count; ++i) {
      input[i] = static_cast<float>(mosaic[i]) / 65535.0F;
    }
    demosaic_ppg(rgb.data(), input.data(), static_cast<int>(width),
                 static_cast<int>(height), filters, 0.0F, 100000);
    for (size_t i = 0; i < pixel_count; ++i) {
      for (size_t channel = 0; channel < 3; ++channel) {
        const auto normalized = std::clamp(rgb[i * 4 + channel], 0.0F, 1.0F);
        output[i * 3 + channel] =
            static_cast<std::uint16_t>(std::lround(normalized * 65535.0F));
      }
    }
    return COMPARE_DEMOSAIC_STATUS_OK;
  } catch (const std::bad_alloc &) {
    return COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY;
  } catch (...) {
    return COMPARE_DEMOSAIC_STATUS_EXCEPTION;
  }
}
