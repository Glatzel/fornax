#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_linear() {
  pre_interpolate();
  lin_interpolate();
  return COMPARE_DEMOSAIC_STATUS_OK;
}
