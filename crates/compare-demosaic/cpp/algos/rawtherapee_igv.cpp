// SPDX-License-Identifier: GPL-3.0-or-later
#include "rawtherapee_adapter.h"

extern "C" std::int32_t run_rawtherapee_igv(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) {
    return run_rawtherapee(mosaic, width, height, filters, output, [](rtengine::RawImageSource& source) {
        source.igv_interpolate(source.W, source.H);
    });
}
