#pragma once

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>

#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif
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
#define for_each_channel(_var, ...) for (size_t _var = 0; _var < DT_PIXEL_SIMD_CHANNELS; ++_var)
#define dt_omp_nontemporal(pointer)

using gboolean = int;
using dt_aligned_pixel_t = float[4];
using std::size_t;

static void pre_median(float*, const float*, int, int, std::uint32_t, int, float) {}

static float* dt_alloc_align_float(size_t count) { return static_cast<float*>(std::malloc(count * sizeof(float))); }

static void dt_free_align(float* pointer) { std::free(pointer); }
