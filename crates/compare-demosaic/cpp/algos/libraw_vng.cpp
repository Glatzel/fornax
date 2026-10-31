#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_vng() {
    pre_interpolate();
    vng_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
