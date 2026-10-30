#include "../status.h"

#include <math.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define restrict
#define DT_ALIGNED_PIXEL
#define DT_RCD_TILESIZE 112
#define DT_PIXEL_SIMD_CHANNELS 4
#define DT_OMP_DECLARE_SIMD(...)
#define DT_OMP_FOR(...)
#define DT_OMP_PRAGMA(...)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define CLIP(x) ((x) < 0.0F ? 0.0F : ((x) > 1.0F ? 1.0F : (x)))
#define FC(row, col, filters) ((filters >> (((((row) << 1) & 14U) + ((col) & 1U)) * 2U)) & 3U)
#define FCNxtrans(row, col, xtrans) 0
#define for_each_channel(_var, ...) for (size_t _var = 0; _var < DT_PIXEL_SIMD_CHANNELS; ++_var)
#define dt_omp_nontemporal(pointer)

typedef int   gboolean;
typedef float dt_aligned_pixel_t[4];

static void
pre_median(float* out, const float* in, int width, int height, uint32_t filters, int passes, float threshold) {
    (void)out;
    (void)in;
    (void)width;
    (void)height;
    (void)filters;
    (void)passes;
    (void)threshold;
}

static float* dt_alloc_align_float(size_t count) { return (float*)malloc(count * sizeof(float)); }

static float* dt_calloc_align_float(size_t count) { return (float*)calloc(count, sizeof(float)); }

static void  dt_free_align(void* pointer) { free(pointer); }
static float sqrf(float value) { return value * value; }

static float interpolatef(float weight, float first, float second) { return weight * (first - second) + second; }

#include "external/darktable/src/iop/demosaicing/ppg.c"
#include "external/darktable/src/iop/demosaicing/rcd.c"

int32_t run_darktable_rcd(const uint16_t* mosaic, uint32_t width, uint32_t height, uint32_t filters, uint16_t* output) {
    const size_t pixel_count = (size_t)width * height;
    for (uint32_t shift = 0; shift < 32; shift += 2) {
        if (((filters >> shift) & 3U) == 3U) {
            filters &= ~(2U << shift);
        }
    }

    float* input = (float*)malloc(pixel_count * sizeof(float));
    float* rgb = (float*)malloc(pixel_count * 4 * sizeof(float));
    if (input == NULL || rgb == NULL) {
        free(input);
        free(rgb);
        return COMPARE_DEMOSAIC_STATUS_ALLOCATION_FAILED;
    }
    for (size_t i = 0; i < pixel_count; ++i) {
        input[i] = (float)mosaic[i] / 65535.0F;
    }

    rcd_demosaic(rgb, input, (int)width, (int)height, filters, 1.0F);
    for (size_t i = 0; i < pixel_count; ++i) {
        for (size_t channel = 0; channel < 3; ++channel) {
            const float value = rgb[i * 4 + channel];
            const float normalized = fmaxf(0.0F, fminf(1.0F, value));
            output[i * 3 + channel] = (uint16_t)lroundf(normalized * 65535.0F);
        }
    }
    free(input);
    free(rgb);
    return COMPARE_DEMOSAIC_STATUS_OK;
}
