#include "../shim.h"
#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif
std::int32_t ReferenceDemosaicer::run_linear() {
    pre_interpolate();
    lin_interpolate();
    return COMPARE_DEMOSAIC_STATUS_OK;
}
