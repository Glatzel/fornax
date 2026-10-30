#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_ppg() {
    pre_interpolate();
    ppg_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
