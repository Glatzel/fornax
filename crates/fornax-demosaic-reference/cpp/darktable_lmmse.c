#include <math.h>
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
#define DT_LMMSE_TILESIZE 136
#define DT_LMMSE_REFINE_2 2
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define CLIP(x) ((x) < 0.0F ? 0.0F : ((x) > 1.0F ? 1.0F : (x)))
#define CLAMPF(a, mn, mx) ((a) >= (mn) ? ((a) <= (mx) ? (a) : (mx)) : (mn))
#define FC(row, col, filters) ((filters >> (((((row) << 1) & 14U) + ((col) & 1U)) * 2U)) & 3U)
#define for_each_channel(_var, ...) for (size_t _var = 0; _var < DT_PIXEL_SIMD_CHANNELS; ++_var)
#define dt_omp_nontemporal(pointer)

#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(expr, value) (expr)
#endif

typedef int   gboolean;
typedef float dt_aligned_pixel_t[4];
typedef int   dt_iop_demosaic_lmmse_t;

static inline float*
dt_alloc_align_float(size_t count) {
    return (float*)malloc(count * sizeof(float));
}

static inline void
dt_free_align(void* pointer) {
    free(pointer);
}

static inline float
sqrf(float value) {
    return value * value;
}

static inline void
pre_median(float* out, const float* in, int width, int height, uint32_t filters, int passes, float threshold) {
    (void)out;
    (void)in;
    (void)width;
    (void)height;
    (void)filters;
    (void)passes;
    (void)threshold;
}

static inline void
dt_print(int level, const char* message, ...) {
    (void)level;
    (void)message;
}

static inline float
median9f(const float* pixels) {
    float p1 = MIN(pixels[1], pixels[2]);
    float p2 = MAX(pixels[1], pixels[2]);
    float p4 = MIN(pixels[4], pixels[5]);
    float p5 = MAX(pixels[4], pixels[5]);
    float p7 = MIN(pixels[7], pixels[8]);
    float p8 = MAX(pixels[7], pixels[8]);
    const float p0 = MIN(pixels[0], p1);
    const float p1a = MAX(pixels[0], p1);
    float p3 = MIN(pixels[3], p4);
    float p4a = MAX(pixels[3], p4);
    float p6 = MIN(pixels[6], p7);
    float p7a = MAX(pixels[6], p7);
    p1 = MIN(p1a, p2);
    p2 = MAX(p1a, p2);
    p4 = MIN(p4a, p5);
    p5 = MAX(p4a, p5);
    p7 = MIN(p7a, p8);
    p8 = MAX(p7a, p8);
    p3 = MAX(p0, p3);
    p5 = MIN(p5, p8);
    p7a = MAX(p4, p7);
    p4 = MIN(p4, p7);
    p6 = MAX(p3, p6);
    p4 = MAX(p1, p4);
    p2 = MIN(p2, p5);
    p4a = MIN(p4, p7a);
    p4 = MIN(p4a, p2);
    p2 = MAX(p4a, p2);
    p4 = MAX(p6, p4);
    return MIN(p2, p4);
}

#include "external/darktable/src/iop/demosaicing/ppg.c"
#include "external/darktable/src/iop/demosaicing/lmmse.c"

int32_t
run_darktable_lmmse(
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

    lmmse_demosaic(
        rgb,
        input,
        (int)width,
        (int)height,
        filters,
        1,
        1.0F
    );
    for (size_t i = 0; i < pixel_count; ++i) {
        for (size_t channel = 0; channel < 3; ++channel) {
            const float value = rgb[i * 4 + channel];
            const float normalized = fmaxf(0.0F, fminf(1.0F, value));
            output[i * 3 + channel] = (uint16_t)lroundf(normalized * 65535.0F);
        }
    }
    free(input);
    free(rgb);
    _cleanup_lmmse_gamma();
    return 0;
}
