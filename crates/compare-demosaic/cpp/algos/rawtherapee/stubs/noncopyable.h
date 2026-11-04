// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

namespace rtengine
{

class NonCopyable
{
  protected:
    NonCopyable() = default;
    ~NonCopyable() = default;

  public:
    NonCopyable(const NonCopyable&) = delete;
    NonCopyable& operator=(const NonCopyable&) = delete;
};

} // namespace rtengine
