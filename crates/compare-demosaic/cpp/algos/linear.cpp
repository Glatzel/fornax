#include "../shim.h"

compare_demosaic_status ReferenceDemosaicer::run_linear() {
  pre_interpolate();
  lin_interpolate();
  return compare_demosaic_ok;
}
