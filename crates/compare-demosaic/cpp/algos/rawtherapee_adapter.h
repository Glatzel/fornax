// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "../status.h"
#include "rawtherapee_compat.h"

#include <new>

template <typename Algorithm>
std::int32_t run_rawtherapee(
    const std::uint16_t* mosaic,
    std::uint32_t width,
    std::uint32_t height,
    std::uint32_t filters,
    std::uint16_t* output,
    Algorithm algorithm
) noexcept {
    if (width < 32 || height < 32) {
        return COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS;
    }

    try {
        const std::uint32_t prefilters = filters;
        for (std::uint32_t shift = 0; shift < 32; shift += 2) {
            if (((filters >> shift) & 3U) == 3U) {
                filters &= ~(2U << shift);
            }
        }

        rtengine::RawImageSource source;
        source.prepare(mosaic, width, height, filters, prefilters);
        algorithm(source);
        source.copy_rgb(output);
        return COMPARE_DEMOSAIC_STATUS_OK;
    } catch (const std::bad_alloc&) {
        return COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY;
    } catch (...) {
        return COMPARE_DEMOSAIC_STATUS_EXCEPTION;
    }
}
