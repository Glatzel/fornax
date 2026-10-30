#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_ahd() {
    pre_interpolate();
    ahd_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
