// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <string>

namespace Glib {

class ustring {
  public:
    ustring() = default;
    ustring(const char*) {}

    template <typename... Args> static ustring compose(const char*, Args&&...) { return {}; }
};

} // namespace Glib

#define M(message) message
