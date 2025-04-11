$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
if ($IsWindows) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../vcpkg/installed/x64-windows-static/lib/pkgconfig
}
if ($IsLinux) {
    sudo apt-get install libraw-dev
}