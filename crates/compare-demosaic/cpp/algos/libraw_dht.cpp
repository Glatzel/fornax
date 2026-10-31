#include "../shim.h"

std::int32_t ReferenceDemosaicer::run_dht() {
    pre_interpolate();
    dht_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
