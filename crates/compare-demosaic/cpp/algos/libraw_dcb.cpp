#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_dcb() {
    pre_interpolate();
    dcb(0, 1);
    return COMPARE_DEMOSAIC_STATUS_OK;
}
