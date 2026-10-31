#include <libraw/libraw.h>

#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <limits>
#include <new>

extern "C" std::int32_t run_darktable_vng(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);

extern "C" std::int32_t run_darktable_lmmse(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);

extern "C" std::int32_t run_darktable_amaze(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);

namespace {

constexpr int STATUS_OK = 0;
constexpr int STATUS_INVALID_DIMENSIONS = 1;
constexpr int STATUS_DIMENSION_OVERFLOW = 2;
constexpr int STATUS_ALLOCATION_FAILED = 3;
constexpr int STATUS_NULL_POINTER = 4;
constexpr int STATUS_OUT_OF_MEMORY = 5;
constexpr int STATUS_EXCEPTION = 6;
constexpr int STATUS_UNKNOWN_ALGORITHM = 7;

class ReferenceDemosaicer final : public LibRaw {
  public:
    std::int32_t prepare(
        const std::uint16_t* mosaic,
        std::uint32_t        width,
        std::uint32_t        height,
        std::uint32_t        filters
    ) {
        const auto pixel_count = static_cast<std::size_t>(width) * height;
        auto* image = static_cast<ushort(*)[4]>(std::calloc(pixel_count, sizeof(ushort[4])));
        if (image == nullptr)
            return STATUS_ALLOCATION_FAILED;

        imgdata.image = image;
        imgdata.sizes.width = static_cast<ushort>(width);
        imgdata.sizes.height = static_cast<ushort>(height);
        imgdata.sizes.iwidth = static_cast<ushort>(width);
        imgdata.sizes.iheight = static_cast<ushort>(height);
        imgdata.idata.colors = 3;
        imgdata.idata.filters = filters;
        for (int row = 0; row < 3; ++row)
            for (int col = 0; col < 3; ++col)
                imgdata.color.rgb_cam[row][col] = row == col ? 1.0F : 0.0F;
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
        return STATUS_OK;
    }

    std::int32_t run(std::uint32_t algorithm) {
        pre_interpolate();
        switch (algorithm) {
            case 1:
                dcb(0, 1);
                return STATUS_OK;
            case 2:
                dht_interpolate();
                return STATUS_OK;
            case 3:
                aahd_interpolate();
                return STATUS_OK;
            default:
                return STATUS_UNKNOWN_ALGORITHM;
        }
    }

    void copy_rgb(std::uint16_t* output) const {
        const auto pixel_count =
            static_cast<std::size_t>(imgdata.sizes.width) * imgdata.sizes.height;
        for (std::size_t index = 0; index < pixel_count; ++index) {
            output[index * 3] = imgdata.image[index][0];
            output[index * 3 + 1] = imgdata.image[index][1];
            output[index * 3 + 2] = imgdata.image[index][2];
        }
    }
};

} // namespace

extern "C" std::int32_t fornax_demosaic_reference(
    std::uint32_t        algorithm,
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) noexcept {
    if (mosaic == nullptr || output == nullptr)
        return STATUS_NULL_POINTER;
    constexpr auto ushort_max = std::numeric_limits<ushort>::max();
    if (width < 3 || height < 3)
        return STATUS_INVALID_DIMENSIONS;
    if (width > ushort_max || height > ushort_max)
        return STATUS_DIMENSION_OVERFLOW;
    if (static_cast<std::size_t>(width) > std::numeric_limits<std::size_t>::max() / height)
        return STATUS_DIMENSION_OVERFLOW;

    if (algorithm == 0)
        return run_darktable_vng(mosaic, width, height, filters, output);
    if (algorithm == 5)
        return run_darktable_lmmse(mosaic, width, height, filters, output);
    if (algorithm == 4)
        return run_darktable_amaze(mosaic, width, height, filters, output);

    try {
        ReferenceDemosaicer reference;
        const auto prepared = reference.prepare(mosaic, width, height, filters);
        if (prepared != STATUS_OK)
            return prepared;
        const auto status = reference.run(algorithm);
        if (status != STATUS_OK)
            return status;
        reference.copy_rgb(output);
        return STATUS_OK;
    } catch (const std::bad_alloc&) {
        return STATUS_OUT_OF_MEMORY;
    } catch (...) {
        return STATUS_EXCEPTION;
    }
}
