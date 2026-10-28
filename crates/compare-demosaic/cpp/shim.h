#pragma once

#include <libraw/libraw.h>

#include <cstdint>

#define COMPARE_DEMOSAIC_STATUS_OK 0
#define COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS 1
#define COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW 2
#define COMPARE_DEMOSAIC_STATUS_ALLOCATION_FAILED 3
#define COMPARE_DEMOSAIC_STATUS_NULL_POINTER 4
#define COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY 5
#define COMPARE_DEMOSAIC_STATUS_EXCEPTION 6
#define COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM 7

class ReferenceDemosaicer final : public LibRaw {
public:
  std::int32_t prepare(const std::uint16_t *mosaic, std::uint32_t width,
                       std::uint32_t height, std::uint32_t filters);
  std::int32_t run_linear();
  void copy_rgb(std::uint16_t *output) const;
};

extern "C" std::int32_t
compare_demosaic_libraw(std::uint32_t algorithm, const std::uint16_t *mosaic,
                        std::uint32_t width, std::uint32_t height,
                        std::uint32_t filters, std::uint16_t *output) noexcept;
