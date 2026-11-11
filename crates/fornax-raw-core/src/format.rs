use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

/// RAW container family recognized by the decoder.
///
/// # Examples
///
/// ```
/// use fornax_raw_core::RawFormat;
///
/// assert_eq!(RawFormat::Dng.to_string(), "DNG");
/// assert_eq!("CR3".parse(), Ok(RawFormat::Cr3));
/// assert_eq!("crm".parse(), Ok(RawFormat::Crm));
/// assert_eq!("nrw".parse(), Ok(RawFormat::Nrw));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, AsRefStr, Display, EnumString, IntoStaticStr)]
#[strum(ascii_case_insensitive, serialize_all = "UPPERCASE")]
pub enum RawFormat {
    /// Adobe Digital Negative.
    Dng,
    /// Canon CR3 or CRM.
    Cr3,
    /// Canon CRM movie raw.
    Crm,
    /// Sony Alpha RAW.
    Arw,
    /// Sony/Zeiss NRW variant of Alpha RAW.
    Nrw,
    /// Nikon Electronic Format.
    Nef,
    /// Fujifilm RAW.
    Raf,
    /// Hasselblad 3FR.
    #[strum(ascii_case_insensitive, serialize = "3FR")]
    Hasselblad3fr,
    /// Panasonic RAW.
    Rw2,
    /// Phase One or Leaf IIQ.
    Iiq,
}
