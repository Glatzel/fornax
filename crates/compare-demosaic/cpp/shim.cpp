#include "shim.h"

#include <cstddef>
#include <cstdlib>
#include <limits>
#include <new>
#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif
extern "C" std::int32_t run_darktable_rcd(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);

std::int32_t ReferenceDemosaicer::prepare(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters
) {
    constexpr auto ushort_max = std::numeric_limits<ushort>::max();
    if (width < 3 || height < 3) {
        return COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS;
    }
    if (width > ushort_max || height > ushort_max) {
        return COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW;
    }
    if (static_cast<std::size_t>(width) > std::numeric_limits<std::size_t>::max() / height) {
        return COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW;
    }

    const auto pixel_count = static_cast<std::size_t>(width) * static_cast<std::size_t>(height);
    auto*      image = static_cast<ushort(*)[4]>(std::calloc(pixel_count, sizeof(ushort[4])));
    if (image == nullptr) {
        return COMPARE_DEMOSAIC_STATUS_ALLOCATION_FAILED;
    }

    // LibRaw::recycle() owns and frees imgdata.image.
    imgdata.image = image;
    imgdata.sizes.width = static_cast<ushort>(width);
    imgdata.sizes.height = static_cast<ushort>(height);
    imgdata.sizes.iwidth = static_cast<ushort>(width);
    imgdata.sizes.iheight = static_cast<ushort>(height);
    imgdata.idata.colors = 3;
    imgdata.idata.filters = filters;
    for (int row = 0; row < 3; ++row) {
        for (int col = 0; col < 3; ++col) {
            imgdata.color.rgb_cam[row][col] = row == col ? 1.0F : 0.0F;
        }
    }
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
    return COMPARE_DEMOSAIC_STATUS_OK;
}

void ReferenceDemosaicer::copy_rgb(std::uint16_t* output) const {
    const auto pixel_count = static_cast<std::size_t>(imgdata.sizes.width) * imgdata.sizes.height;
    for (std::size_t index = 0; index < pixel_count; ++index) {
        output[index * 3] = imgdata.image[index][0];
        output[index * 3 + 1] = imgdata.image[index][1];
        output[index * 3 + 2] = imgdata.image[index][2];
    }
}

extern "C" std::int32_t compare_demosaic_reference(
    std::uint32_t        reference,
    std::uint32_t        algorithm,
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) noexcept {
    if (mosaic == nullptr || output == nullptr) {
        return COMPARE_DEMOSAIC_STATUS_NULL_POINTER;
    }
    constexpr auto ushort_max = std::numeric_limits<ushort>::max();
    if (width < 3 || height < 3) {
        return COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS;
    }
    if (width > ushort_max || height > ushort_max) {
        return COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW;
    }
    if (static_cast<std::size_t>(width) > std::numeric_limits<std::size_t>::max() / height) {
        return COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW;
    }
    if (reference == COMPARE_DEMOSAIC_REFERENCE_DARKTABLE) {
        switch (algorithm) {
            case 0:
                return run_darktable_ppg(mosaic, width, height, filters, output);
            case 1:
                return run_darktable_rcd(mosaic, width, height, filters, output);
            case 2:
                return run_darktable_vng(mosaic, width, height, filters, output);
            case 3:
                return run_darktable_amaze(mosaic, width, height, filters, output);
            case 4:
                return run_darktable_lmmse(mosaic, width, height, filters, output);
            default:
                return COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM;
        }
    }
    if (reference != COMPARE_DEMOSAIC_REFERENCE_LIBRAW) {
        return COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM;
    }

    try {
        ReferenceDemosaicer reference;
        const auto          prepared = reference.prepare(mosaic, width, height, filters);
        if (prepared != COMPARE_DEMOSAIC_STATUS_OK) {
            return prepared;
        }

        auto status = COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM;
        switch (algorithm) {
            case 0:
                status = reference.run_linear();
                break;
            case 1:
                status = reference.run_ppg();
                break;
            case 2:
                status = reference.run_ahd();
                break;
            case 3:
                status = reference.run_vng();
                break;
            case 4:
                status = reference.run_dcb();
                break;
            case 5:
                status = reference.run_dht();
                break;
            case 6:
                status = reference.run_aahd();
                break;
            default:
                break;
        }
        if (status != COMPARE_DEMOSAIC_STATUS_OK) {
            return status;
        }

        reference.copy_rgb(output);
        return COMPARE_DEMOSAIC_STATUS_OK;
    } catch (const std::bad_alloc&) {
        return COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY;
    } catch (...) {
        return COMPARE_DEMOSAIC_STATUS_EXCEPTION;
    }
}
