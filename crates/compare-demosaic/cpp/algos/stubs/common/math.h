#pragma once
#if defined(_MSC_VER) && !defined(__clang__)
#define __builtin_expect(x, y) (x)
#endif