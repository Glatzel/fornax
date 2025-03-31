#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("../../vcpkg/installed/x64-windows-static-md/include/libraw/libraw.h");
    }
}
