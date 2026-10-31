#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_aahd() {
    pre_interpolate();
    aahd_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
