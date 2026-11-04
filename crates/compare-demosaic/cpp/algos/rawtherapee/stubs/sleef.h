// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cmath>

inline float xexpf(float value) { return std::exp(value); }
inline float xlogf(float value) { return std::log(value); }
inline float xexpfNoCheck(float value) { return std::exp(value); }
inline float xlogfNoCheck(float value) { return std::log(value); }
inline float xdivf(float numerator, float denominator) { return numerator / denominator; }
inline float xdiv2f(float value) { return xdivf(value, 2.0F); }
inline float xmul2f(float value) { return value * 2.0F; }
