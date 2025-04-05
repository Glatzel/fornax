pub struct LibrawVersion {
    major: u32,
    minor: u32,
    patch: u32,
}
impl LibrawVersion {
    pub(crate) fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
    pub fn major(&self) -> u32 {
        self.major
    }
    pub fn minor(&self) -> u32 {
        self.minor
    }
    pub fn patch(&self) -> u32 {
        self.patch
    }
}
pub static LIBRAW_VERSION: std::sync::LazyLock<LibrawVersion> = std::sync::LazyLock::new(|| {
    LibrawVersion::new(
        libraw_sys::LIBRAW_MAJOR_VERSION,
        libraw_sys::LIBRAW_MINOR_VERSION,
        libraw_sys::LIBRAW_PATCH_VERSION,
    )
});
