//! Universal RAW image model, metadata, geometry, CFA, and validation.

mod cfa;
mod decoded_raw;
mod error;
mod geometry;
mod metadata;
mod plane;
mod raw_image;

pub use cfa::{BayerLayout, BayerPattern, BayerSlot, CfaColor, CfaShape};
pub use decoded_raw::DecodedRaw;
pub use error::{MetadataError, RawValidationError};
pub use geometry::{ActiveArea, PixelCoord, PixelRect, PixelSize, StoredGeometry};
pub use metadata::{
    AuthorityClass, BlackLevels, BlackTile, CameraIdentity, CaptureMetadata, ColorMatrices,
    CommonMetadata, Confidence, CoordinateSpace, DecoderProvenance, DefaultCrop,
    ExtensibleMetadata, MetadataConflict, MetadataEntry, MetadataField, MetadataKey, MetadataUnit,
    MetadataValue, Orientation, OriginalValue, Provenance, ProviderId, Rational, SourceKind,
    Sourced, TypedMetadataField, ValidatedMetadata, WhiteBalance, WhiteLevels, resolve_sourced,
};
pub use plane::{F32Plane, SampleFormat, SampleSpec, StoredPlane, U8Plane, U16Plane, U32Plane};
pub use raw_image::{RawImage, into_preprocessed};
