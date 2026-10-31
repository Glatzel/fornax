#include <limits.h>
#include <math.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define DT_ALIGNED_PIXEL
#define DT_PIXEL_SIMD_CHANNELS 4
#define DT_OMP_DECLARE_SIMD(...)
#define DT_OMP_FOR(...)
#define DT_OMP_PRAGMA(...)
#define DT_DEBUG_ALWAYS 0
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define FILTERS_ARE_4BAYER(filters) (0)
#define for_each_channel(_var, ...) for (size_t _var = 0; _var < DT_PIXEL_SIMD_CHANNELS; ++_var)
#define dt_omp_nontemporal(pointer)

#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(expr, value) (expr)
#endif

typedef int   gboolean;
typedef float dt_aligned_pixel_t[4];

static inline int
fcol(int row, int col, uint32_t filters, const uint8_t (*xtrans)[6]) {
    (void)xtrans;
    return (filters >> (((((unsigned)row << 1) & 14U) + ((unsigned)col & 1U)) * 2U)) & 3U;
}

static inline void*
dt_alloc_aligned_impl(size_t size) {
    return malloc(size);
}

static inline void
dt_free_align(void* pointer) {
    free(pointer);
}

static inline void
dt_iop_image_copy(float* destination, const float* source, size_t count) {
    memcpy(destination, source, count * sizeof(float));
}

static inline void
dt_vector_clipneg(dt_aligned_pixel_t pixel) {
    for (size_t channel = 0; channel < 4; ++channel)
        pixel[channel] = fmaxf(0.0F, pixel[channel]);
}

static inline void
dt_print(int level, const char* message, ...) {
    (void)level;
    (void)message;
}

#define dt_alloc_aligned(size) dt_alloc_aligned_impl(size)

#include "external/darktable/src/iop/demosaicing/vng.c"

int32_t
run_darktable_vng(
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
    float* input = (float*)calloc(pixel_count, sizeof(float));
    float* rgb = (float*)malloc(pixel_count * 4 * sizeof(float));
    if (input == NULL || rgb == NULL) {
        free(input);
        free(rgb);
        return 3;
    }
    for (size_t i = 0; i < pixel_count; ++i)
        input[i] = (float)mosaic[i] / 65535.0F;

    vng_interpolate(rgb, input, (int)width, (int)height, filters, NULL, 0);
    for (size_t i = 0; i < pixel_count; ++i) {
        for (size_t channel = 0; channel < 3; ++channel) {
            const float value = rgb[i * 4 + channel];
            const float normalized = fmaxf(0.0F, fminf(1.0F, value));
            output[i * 3 + channel] = (uint16_t)lroundf(normalized * 65535.0F);
        }
    }
    free(input);
    free(rgb);
    return 0;
}
