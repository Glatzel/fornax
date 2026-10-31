#pragma once

#include <libraw/libraw.h>

#include <cstdint>

#include "status.h"
#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif
#define COMPARE_DEMOSAIC_REFERENCE_LIBRAW 0

class ReferenceDemosaicer final : public LibRaw {
  public:
    std::int32_t prepare(const std::uint16_t* mosaic, std::uint32_t width, std::uint32_t height, std::uint32_t filters);
    std::int32_t run_linear();
    std::int32_t run_ppg();
    std::int32_t run_ahd();
    std::int32_t run_vng();
    std::int32_t run_dcb();
    std::int32_t run_dht();
    std::int32_t run_aahd();
    void         copy_rgb(std::uint16_t* output) const;
};

extern "C" std::int32_t compare_demosaic_reference(
    std::uint32_t        reference,
    std::uint32_t        algorithm,
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) noexcept;

std::int32_t run_darktable_ppg(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) noexcept;

#ifdef __cplusplus
extern "C" {
#endif
std::int32_t run_darktable_vng(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
) noexcept;

#ifdef __cplusplus
extern "C" {
#endif
std::int32_t run_darktable_amaze(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);

std::int32_t run_darktable_lmmse(
    const std::uint16_t* mosaic,
    std::uint32_t        width,
    std::uint32_t        height,
    std::uint32_t        filters,
    std::uint16_t*       output
);
#ifdef __cplusplus
}
#endif
#ifdef __cplusplus
}
#endif
