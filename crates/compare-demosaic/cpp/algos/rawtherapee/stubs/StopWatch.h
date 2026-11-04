// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#ifdef BENCHMARK
#define BENCHFUN
#define BENCHFUNMICRO
#else
#define BENCHFUN
#define BENCHFUNMICRO
#endif

class StopWatch
{
  public:
    explicit StopWatch(const char*, bool = false) {}
};
