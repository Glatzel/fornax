#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>

#define AMAZETS 160
#define G_BEGIN_DECLS
#define G_END_DECLS
#define DT_OMP_PRAGMA(...)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define LIM(x, min, max) MAX(min, MIN(x, max))
#define ULIM(x, y, z) ((y) < (z) ? LIM(x, y, z) : LIM(x, z, y))
#define FC(row, col, filters) ((filters >> (((((row) << 1) & 14U) + ((col) & 1U)) * 2U)) & 3U)

static inline float
sqrf(float value) {
    return value * value;
}

static inline float
interpolatef(float weight, float first, float second) {
    return weight * (first - second) + second;
}

#include "external/darktable/src/iop/demosaicing/amaze.cc"

extern "C" int32_t
run_darktable_amaze(
    const uint16_t* mosaic,
    uint32_t        width,
    uint32_t        height,
    uint32_t        filters,
    uint16_t*       output
) {
    const size_t pixel_count = (size_t)width * height;
    for (uint32_t shift = 0; shift < 32; shift += 2) {
        if (((filters >> shift) & 3U) == 3U)
            filters &= ~(2U << shift);
    }
    float* input = (float*)std::calloc(pixel_count, sizeof(float));
    float* rgb = (float*)malloc(pixel_count * 4 * sizeof(float));
    if (input == nullptr || rgb == nullptr) {
        free(input);
        free(rgb);
        return 3;
    }
    for (size_t i = 0; i < pixel_count; ++i)
        input[i] = (float)mosaic[i] / 65535.0F;

    amaze_demosaic(input, rgb, (int)width, (int)height, filters, 1.0F);
    for (size_t i = 0; i < pixel_count; ++i) {
        for (size_t channel = 0; channel < 3; ++channel) {
            const float value = rgb[i * 4 + channel];
            const float normalized = std::fmax(0.0F, std::fmin(1.0F, value));
            output[i * 3 + channel] = (uint16_t)std::lround(normalized * 65535.0F);
        }
    }
    free(input);
    free(rgb);
    return 0;
}
