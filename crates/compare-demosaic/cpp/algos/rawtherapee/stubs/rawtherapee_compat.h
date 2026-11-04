/*
 * Compatibility interface derived from RawTherapee 5.13.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
#pragma once

#include <algorithm>
#include <array>
#include <climits>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <limits>
#include <memory>
#include <mutex>
#include <vector>

#include "rtgui/multilangmgr.h"

constexpr unsigned int ARRAY2D_CLEAR_DATA = 1;
constexpr unsigned int ARRAY2D_BYREFERENCE = 2;
constexpr int          LUT_CLIP_BELOW = 1;
constexpr int          LUT_CLIP_ABOVE = 2;

namespace rtengine {

enum eSensorType {
    ST_NONE,
    ST_BAYER,
    ST_FUJI_XTRANS,
    ST_FOVEON,
};

class LUTf {
  public:
    LUTf() = default;

    explicit LUTf(std::size_t size, int = 0) : values(size, 0.0F) {}

    float&       operator[](std::size_t index) { return values[clamped(index)]; }
    float&       operator[](int index) { return values[clamped(static_cast<std::ptrdiff_t>(index))]; }
    float        operator[](float index) { return values[clamped(static_cast<std::ptrdiff_t>(index))]; }
    const float& operator[](int index) const { return values[clamped(static_cast<std::ptrdiff_t>(index))]; }
    float        operator[](float index) const { return values[clamped(static_cast<std::ptrdiff_t>(index))]; }

    void makeIdentity(float maximum = 65535.0F) {
        for (std::size_t i = 0; i < values.size(); ++i) {
            values[i] = maximum * static_cast<float>(i) / static_cast<float>(values.size() - 1);
        }
    }

  private:
    std::size_t clamped(std::ptrdiff_t index) const {
        if (index <= 0) {
            return 0;
        }
        const auto upper = static_cast<std::ptrdiff_t>(values.size() - 1);
        return static_cast<std::size_t>(std::min(index, upper));
    }

    std::vector<float> values;
};

class Color {
  public:
    inline static std::array<float, 65536> cachef{};
    inline static std::array<float, 65536> cachefy{};
    inline static LUTf                     gammatab_24_17a{65536};
    inline static LUTf                     igammatab_24_17{65536};

    static void init() {
        static std::once_flag initialized;
        std::call_once(initialized, [] {
            constexpr double eps = 216.0 / 24389.0;
            constexpr double kappa = 24389.0 / 27.0;
            constexpr double maximum = 65535.0;
            constexpr int    epsilon_maximum = static_cast<int>(maximum * eps);
            for (int i = 0; i < 65536; ++i) {
                if (i <= epsilon_maximum) {
                    cachef[i] = 327.68F * static_cast<float>((kappa * i / maximum + 16.0) / 116.0);
                    cachefy[i] = 327.68F * static_cast<float>(kappa * i / maximum);
                } else {
                    cachef[i] = 327.68F * std::cbrt(static_cast<float>(i) / 65535.0F);
                    cachefy[i] = 327.68F * (116.0F * std::cbrt(static_cast<float>(i) / 65535.0F) - 16.0F);
                }
                const float normalized = static_cast<float>(i) / 65535.0F;
                gammatab_24_17a[i] = normalized <= 0.001867F
                                         ? normalized * 17.0F
                                         : 1.044445F * std::exp(std::log(normalized) / 2.4F) - 0.044445F;
                igammatab_24_17[i] = 65535.0F * (normalized <= 0.031746F
                                                     ? normalized / 17.0F
                                                     : std::exp(std::log((normalized + 0.044445F) / 1.044445F) * 2.4F));
            }
        });
    }

    static void RGB2Lab(
        float*      red,
        float*      green,
        float*      blue,
        float*      lightness,
        float*      a,
        float*      b,
        const float matrix[3][3],
        int         width
    ) {
        init();
        for (int i = 0; i < width; ++i) {
            const float xyz[3] = {
                matrix[0][0] * red[i] + matrix[0][1] * green[i] + matrix[0][2] * blue[i],
                matrix[1][0] * red[i] + matrix[1][1] * green[i] + matrix[1][2] * blue[i],
                matrix[2][0] * red[i] + matrix[2][1] * green[i] + matrix[2][2] * blue[i],
            };
            const float fx = xyz_to_lab(xyz[0]);
            const float fy = xyz_to_lab(xyz[1]);
            const float fz = xyz_to_lab(xyz[2]);
            lightness[i] = xyz_to_lab_lightness(xyz[1]);
            a[i] = 500.0F * (fx - fy);
            b[i] = 200.0F * (fy - fz);
        }
    }

  private:
    static float xyz_to_lab(float value) {
        constexpr double eps = 216.0 / 24389.0;
        constexpr double kappa = 24389.0 / 27.0;
        if (value < 0.0F) {
            return 327.68F * static_cast<float>((kappa * value / 65535.0 + 16.0) / 116.0);
        }
        if (value > 65535.0F) {
            return 327.68F * std::cbrt(value / 65535.0F);
        }
        const int index = static_cast<int>(value);
        if (index <= static_cast<int>(65535.0 * eps)) {
            return 327.68F * static_cast<float>((kappa * index / 65535.0 + 16.0) / 116.0);
        }
        return 327.68F * std::cbrt(static_cast<float>(index) / 65535.0F);
    }

    static float xyz_to_lab_lightness(float value) {
        constexpr double eps = 216.0 / 24389.0;
        constexpr double kappa = 24389.0 / 27.0;
        if (value < 0.0F) {
            return 327.68F * static_cast<float>(kappa * value / 65535.0);
        }
        if (value > 65535.0F) {
            return 327.68F * static_cast<float>(116.0 * std::cbrt(value / 65535.0) - 16.0);
        }
        const int index = static_cast<int>(value);
        if (index <= static_cast<int>(65535.0 * eps)) {
            return 327.68F * static_cast<float>(kappa * index / 65535.0);
        }
        return 327.68F * static_cast<float>(116.0 * std::cbrt(static_cast<float>(index) / 65535.0) - 16.0);
    }
};

template <typename T> class array2D {
  public:
    array2D() = default;

    array2D(int width, int height, unsigned int flags = 0) { resize(width, height, flags); }

    array2D(int width, int height, T** source, unsigned int flags = 0) {
        resize(width, height, ARRAY2D_BYREFERENCE);
        if (!(flags & ARRAY2D_BYREFERENCE)) {
            for (int row = 0; row < height; ++row) {
                std::copy(source[row], source[row] + width, rows[static_cast<std::size_t>(row)]);
            }
        }
    }

    void resize(int width, int height, unsigned int flags = 0) {
        width_ = width;
        storage_.assign(static_cast<std::size_t>(width) * static_cast<std::size_t>(height), T{});
        rows.resize(static_cast<std::size_t>(height));
        for (int row = 0; row < height; ++row) {
            rows[static_cast<std::size_t>(row)] = storage_.data() + static_cast<std::size_t>(row) * width;
        }
        if (flags & ARRAY2D_CLEAR_DATA) {
            std::fill(storage_.begin(), storage_.end(), T{});
        }
    }

    void operator()(int width, int height, unsigned int flags = 0, int = 0) { resize(width, height, flags); }

    T* operator[](int row) { return rows[static_cast<std::size_t>(row)]; }

    const T* operator[](int row) const { return rows[static_cast<std::size_t>(row)]; }

    operator T**() { return rows.data(); }
    operator const T* const*() const { return rows.data(); }
    operator T*() { return storage_.data(); }
    operator const T*() const { return storage_.data(); }

  private:
    int             width_ = 0;
    std::vector<T*> rows;
    std::vector<T>  storage_;
};

struct ImageMatrices {
    double rgb_cam[3][3] = {
        {1.0, 0.0, 0.0},
        {0.0, 1.0, 0.0},
        {0.0, 0.0, 1.0},
    };
};

class ProgressListener {
  public:
    virtual ~ProgressListener() = default;
    virtual void setProgressStr(const Glib::ustring&) {}
    virtual void setProgress(double) {}
};

class RawImage {
  public:
    unsigned int filters = 0;
    unsigned int prefilters = 0;

    unsigned int get_filters() const { return filters; }
    eSensorType  getSensorType() const { return ST_BAYER; }

    bool         ISRED(unsigned row, unsigned col) const { return FC(row, col) == 0; }
    bool         ISGREEN(unsigned row, unsigned col) const { return FC(row, col) == 1; }
    bool         ISBLUE(unsigned row, unsigned col) const { return FC(row, col) == 2; }
    unsigned int XTRANSFC(unsigned, unsigned) const { return 0; }

    unsigned int FC(unsigned row, unsigned col) const {
        return (filters >> ((((row << 1) & 14U) + (col & 1U)) * 2U)) & 3U;
    }
};

class RawImageSource {
  public:
    RawImageSource() : ri(&raw_image) {
        Color::init();
        imatrices.rgb_cam[0][0] = 1.0;
        imatrices.rgb_cam[1][1] = 1.0;
        imatrices.rgb_cam[2][2] = 1.0;
    }

    void prepare(
        const std::uint16_t* mosaic,
        std::uint32_t        width,
        std::uint32_t        height,
        std::uint32_t        filters,
        std::uint32_t        prefilters
    ) {
        W = static_cast<int>(width);
        H = static_cast<int>(height);
        rawData(W, H);
        red(W + 2, H + 2);
        green(W + 2, H + 2);
        blue(W + 2, H + 2);

        raw_image.filters = filters;
        raw_image.prefilters = prefilters;
        for (int row = 0; row < H; ++row) {
            for (int col = 0; col < W; ++col) {
                rawData[row][col] = static_cast<float>(mosaic[static_cast<std::size_t>(row) * width + col]);
            }
        }
    }

    void copy_rgb(std::uint16_t* output) const {
        const auto pixel_count = static_cast<std::size_t>(W) * static_cast<std::size_t>(H);
        for (std::size_t index = 0; index < pixel_count; ++index) {
            const int row = static_cast<int>(index / static_cast<std::size_t>(W));
            const int col = static_cast<int>(index % static_cast<std::size_t>(W));
            output[index * 3] = to_u16(red[row][col]);
            output[index * 3 + 1] = to_u16(green[row][col]);
            output[index * 3 + 2] = to_u16(blue[row][col]);
        }
    }

    unsigned int FC(int row, int col) const { return raw_image.FC(row, col); }

    inline void convert_row_to_YIQ(
        const float* red_values,
        const float* green_values,
        const float* blue_values,
        float*       y,
        float*       i,
        float*       q,
        int          width
    );
    inline void convert_row_to_RGB(
        float*       red_values,
        float*       green_values,
        float*       blue_values,
        const float* y,
        const float* i,
        const float* q,
        int          width
    );
    inline void convert_to_RGB(float& red_value, float& green_value, float& blue_value, float y, float i, float q);
    inline void interpolate_row_g(float* horizontal, float* vertical, int row);
    inline void interpolate_row_rb(
        float* red_values,
        float* blue_values,
        float* previous_green,
        float* current_green,
        float* next_green,
        int    row
    );

    void border_interpolate(
        int                   winw,
        int                   winh,
        int                   border,
        const array2D<float>& input,
        array2D<float>&       red_out,
        array2D<float>&       green_out,
        array2D<float>&       blue_out
    );
    void nodemosaic(bool monochrome);
    void igv_interpolate(int winw, int winh);
    void dcb_initTileLimits(int& col_min, int& row_min, int& col_max, int& row_max, int x, int y, int border);
    void fill_raw(float (*cache)[3], int x, int y, float** input);
    void fill_border(float (*cache)[3], int border, int x, int y);
    void copy_to_buffer(float (*buffer)[2], float (*image)[3]);
    void restore_from_buffer(float (*image)[3], float (*buffer)[2]);
    void dcb_hid(float (*image)[3], int x, int y);
    void dcb_color(float (*image)[3], int x, int y);
    void dcb_hid2(float (*image)[3], int x, int y);
    void dcb_map(float (*image)[3], std::uint8_t* map, int x, int y);
    void dcb_correction(float (*image)[3], std::uint8_t* map, int x, int y);
    void dcb_pp(float (*image)[3], int x, int y);
    void dcb_correction2(float (*image)[3], std::uint8_t* map, int x, int y);
    void dcb_refinement(float (*image)[3], std::uint8_t* map, int x, int y);
    void dcb_color_full(float (*image)[3], int x, int y, float (*chroma)[2]);
    void dcb_demosaic(int iterations, bool enhance);
    void ahd_demosaic();
    void amaze_demosaic_RT(
        int                   winx,
        int                   winy,
        int                   winw,
        int                   winh,
        const array2D<float>& input,
        array2D<float>&       red_out,
        array2D<float>&       green_out,
        array2D<float>&       blue_out,
        std::size_t           chunk_size,
        bool                  measure
    );
    void eahd_demosaic();
    void hphd_demosaic();
    void lmmse_interpolate_omp(
        int                   winw,
        int                   winh,
        const array2D<float>& input,
        array2D<float>&       red_out,
        array2D<float>&       green_out,
        array2D<float>&       blue_out,
        int                   iterations
    );
    void refinement(int passes);
    void rcd_demosaic(std::size_t chunk_size, bool measure);
    void vng4_demosaic(
        const array2D<float>& input,
        array2D<float>&       red_out,
        array2D<float>&       green_out,
        array2D<float>&       blue_out
    );
    void interpolate_row_rb_mul_pp(
        const array2D<float>& input,
        float*                red_out,
        float*                blue_out,
        float*                previous_green,
        float*                current_green,
        float*                next_green,
        int                   row,
        float                 red_mul,
        float                 green_mul,
        float                 blue_mul,
        int                   x1,
        int                   width,
        int                   skip
    );

    int               W = 0;
    int               H = 0;
    RawImage          raw_image;
    RawImage*         ri;
    array2D<float>    rawData;
    array2D<float>    red;
    array2D<float>    green;
    array2D<float>    blue;
    ImageMatrices     imatrices;
    ProgressListener* plistener = nullptr;
    double            initialGain = 1.0;
    int               border = 4;

  private:
    static std::uint16_t to_u16(float value) {
        const float clamped = std::clamp(value, 0.0F, 65535.0F);
        return static_cast<std::uint16_t>(std::lround(clamped));
    }
};

} // namespace rtengine
