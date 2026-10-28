#pragma once

#include <libraw/libraw.h>

#include <cstdint>

enum compare_demosaic_status : std::int32_t {
  compare_demosaic_ok = 0,
  compare_demosaic_invalid_dimensions = 1,
  compare_demosaic_dimension_overflow = 2,
  compare_demosaic_allocation_failed = 3,
  compare_demosaic_null_pointer = 4,
  compare_demosaic_out_of_memory = 5,
  compare_demosaic_exception = 6,
  compare_demosaic_unknown_algorithm = 7,
};

class ReferenceDemosaicer final : public LibRaw {
public:
  compare_demosaic_status prepare(const std::uint16_t *mosaic,
                                  std::uint32_t width, std::uint32_t height,
                                  std::uint32_t filters);
  compare_demosaic_status run_linear();
  void copy_rgb(std::uint16_t *output) const;
};

extern "C" std::int32_t
compare_demosaic_libraw(std::uint32_t algorithm,
                        const std::uint16_t *mosaic, std::uint32_t width,
                        std::uint32_t height, std::uint32_t filters,
                        std::uint16_t *output) noexcept;
