use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

/// RAW container family recognized by the decoder.
///
/// # Examples
///
/// ```
/// use fornax_decoder::SourceFormat;
///
/// assert_eq!(SourceFormat::Dng.to_string(), "DNG");
/// assert_eq!("CR3".parse(), Ok(SourceFormat::Cr3));
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq, AsRefStr, Display, EnumString, IntoStaticStr)]
pub enum SourceFormat {
    /// Adobe Digital Negative.
    #[strum(serialize = "DNG")]
    Dng,
    /// Canon CR3 or CRM.
    #[strum(serialize = "CR3")]
    Cr3,
    /// Sony Alpha RAW.
    #[strum(serialize = "ARW")]
    Arw,
    /// Nikon Electronic Format.
    #[strum(serialize = "NEF")]
    Nef,
    /// Fujifilm RAW.
    #[strum(serialize = "RAF")]
    Raf,
    /// Hasselblad 3FR.
    #[strum(serialize = "3FR")]
    Hasselblad3fr,
    /// Panasonic RAW.
    #[strum(serialize = "RW2")]
    Rw2,
    /// Phase One or Leaf IIQ.
    #[strum(serialize = "IIQ")]
    Iiq,
}
