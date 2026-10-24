//! Backend-independent types for decoding, preprocessing, and demosaicing RAW
//! images.
//!
//! [`RawImage`] is the universal container: it combines compact sensor samples
//! with immutable validated metadata, provenance, and processing state.
//! Standard preprocessing consumes that same container and records a finalized
//! state so corrections cannot silently run twice.
//!
//! # Examples
//!
//! ```
//! use fornax_core::{ActiveArea, BayerLayout, BayerPattern, CfaColor, PixelCoord, PixelSize};
//!
//! let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 1, 1, 4, 4).unwrap();
//! let layout = BayerLayout::new(BayerPattern::Rggb, area);
//! assert_eq!(layout.color_at(PixelCoord::new(1, 1)), Ok(CfaColor::Red));
//! ```

#![warn(missing_docs)]

mod cfa;
mod error;
mod geometry;
mod metadata;
mod plane;
mod raw_image;

pub use cfa::{BayerLayout, BayerPattern, BayerSlot, CfaColor, CfaShape};
pub use error::{MetadataError, RawValidationError};
pub use geometry::{ActiveArea, PixelCoord, PixelRect, PixelSize, StoredGeometry};
pub use metadata::{
    AuthorityClass, BlackLevels, BlackTile, CameraIdentity, CaptureMetadata, ColorMatrices,
    CommonMetadata, Confidence, CoordinateSpace, DefaultCrop, ExtensibleMetadata, MetadataConflict,
    MetadataEntry, MetadataField, MetadataKey, MetadataUnit, MetadataValue, Orientation,
    OriginalValue, Provenance, ProviderId, Rational, SourceKind, Sourced, TypedMetadataField,
    ValidatedMetadata, WhiteBalance, WhiteLevels, resolve_sourced,
};
pub use plane::{F32Plane, SampleEncoding, SampleFormat, StoredPlane, U8Plane, U16Plane, U32Plane};
pub use raw_image::{
    DecodedRaw, DecoderProvenance, MetadataBasis, OperationProvenance, PlaneStorage, RawImage,
    RawImageState, into_preprocessed,
};
