#include "../shim.h"
#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif
std::int32_t ReferenceDemosaicer::run_vng() {
    pre_interpolate();
    vng_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}

std::int32_t ReferenceDemosaicer::run_vng4() {
    imgdata.params.four_color_rgb = 1;
    pre_interpolate();
    vng_interpolate();
    const auto pixel_count =
        static_cast<std::size_t>(imgdata.sizes.width) * imgdata.sizes.height;
    for (std::size_t i = 0; i < pixel_count; ++i) {
        imgdata.image[i][1] = (imgdata.image[i][1] + imgdata.image[i][3]) >> 1;
    }
    imgdata.idata.colors = 3;
    return COMPARE_DEMOSAIC_STATUS_OK;
}
