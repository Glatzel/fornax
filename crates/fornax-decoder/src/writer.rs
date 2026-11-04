/// DNG compression modes written in the first implementation stage.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngCompression;
///
/// assert_eq!(DngCompression::default(), DngCompression::LosslessJpeg);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DngCompression {
    /// Uncompressed strip storage.
    Uncompressed,
    /// DNG lossless JPEG-92 strip or tile storage.
    #[default]
    LosslessJpeg,
}

/// Four-component DNG specification version stored in `DNGVersion`.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngVersion;
///
/// assert_eq!(DngVersion::latest().minor, 7);
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DngVersion {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,
    /// Patch version.
    pub patch: u8,
    /// Build version.
    pub build: u8,
}

impl DngVersion {
    /// Returns DNG specification 1.7.1.0, September 2023.
    pub const fn latest() -> Self {
        Self {
            major: 1,
            minor: 7,
            patch: 1,
            build: 0,
        }
    }

    /// Creates a DNG specification version.
    pub const fn new(major: u8, minor: u8, patch: u8, build: u8) -> Self {
        Self {
            major,
            minor,
            patch,
            build,
        }
    }
}

impl Default for DngVersion {
    fn default() -> Self { Self::latest() }
}

/// DNG writer configuration.
///
/// # Examples
///
/// ```
/// use fornax_decoder::{DngCompression, DngVersion, DngWriteOptions};
///
/// let options = DngWriteOptions::default();
/// assert_eq!(options.compression(), DngCompression::LosslessJpeg);
/// assert_eq!(options.version(), DngVersion::latest());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DngWriteOptions {
    version: DngVersion,
    compression: DngCompression,
}

impl Default for DngWriteOptions {
    fn default() -> Self { Self::new(DngCompression::default()) }
}

impl DngWriteOptions {
    /// Creates writer options for the current DNG specification and requested
    /// compression mode.
    pub const fn new(compression: DngCompression) -> Self {
        Self {
            version: DngVersion::latest(),
            compression,
        }
    }

    /// Selects a DNG specification version.
    pub const fn with_version(mut self, version: DngVersion) -> Self {
        self.version = version;
        self
    }

    /// Returns the selected compression mode.
    pub const fn compression(self) -> DngCompression { self.compression }

    /// Returns the selected DNG specification version.
    pub const fn version(self) -> DngVersion { self.version }
}
