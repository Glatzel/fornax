//! LibRaw decoder backed by the workspace's private safe wrapper.

mod snapshot;

use std::num::{NonZeroU32, NonZeroU64};
use std::path::Path;

use image::ImageBuffer;
use libraw::{Libraw, LibrawError, LibrawErrorCode};
use snapshot::{RawBuffer, RawSnapshot, snapshot};

use super::IRawDecoder;
use crate::error::{DecodeError, RawValidationError};
use crate::geometry::{ActiveArea, PixelSize, StoredGeometry};
use crate::metadata::{
    AuthorityClass, BayerLayout, BayerPattern, BlackLevels, CameraIdentity, CfaColor, CfaShape,
    CommonMetadata, Confidence, CoordinateSpace, DecoderProvenance, DefaultCrop,
    ExtensibleMetadata, MetadataEntry, MetadataKey, MetadataUnit, MetadataValue, Orientation,
    Provenance, ProviderId, Rational, SourceKind, Sourced, ValidatedMetadata, WhiteLevels,
};
use crate::{DecodedRaw, SampleFormat, SampleSpec, StoredPlane};

#[cfg(test)]
const LIBRAW_RAW2_IMAGE_PROGRESS: u32 = 1 << 4;
const LIBRAW_POSTPROCESS_PROGRESS_MASK: u32 = 0x000f_fff0;

/// A decoder for conventional 2x2 RGB Bayer images through LibRaw.
///
/// The decoder stops after LibRaw's raw-domain `unpack` operation. It preserves
/// stored sensor samples and leaves preprocessing and demosaicing to `fornax`.
///
/// # Examples
///
/// ```
/// use fornax::decoder::{IRawDecoder, LibRawDecoder};
///
/// let decoder = LibRawDecoder::new();
/// let _: &dyn IRawDecoder<[u8]> = &decoder;
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LibRawDecoder;

impl LibRawDecoder {
    /// Creates a LibRaw decoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::decoder::LibRawDecoder;
    ///
    /// assert_eq!(LibRawDecoder::new(), LibRawDecoder);
    /// ```
    pub const fn new() -> Self { Self }
}

impl IRawDecoder<Path> for LibRawDecoder {
    fn decode(&self, input: &Path) -> Result<DecodedRaw, DecodeError> {
        ensure_supported_path(input)?;
        let revision = Libraw::version().ok();
        let context = Libraw::try_new(None).map_err(translate_libraw_error)?;
        context.open_file(input).map_err(translate_libraw_error)?;
        context.unpack().map_err(translate_libraw_error)?;
        let snapshot = snapshot(&context)?;
        map_snapshot(snapshot, revision)
    }
}

impl IRawDecoder<[u8]> for LibRawDecoder {
    fn decode(&self, input: &[u8]) -> Result<DecodedRaw, DecodeError> {
        let revision = Libraw::version().ok();
        let context = Libraw::try_new(None).map_err(translate_libraw_error)?;
        context.open_buffer(input).map_err(translate_libraw_error)?;
        context.unpack().map_err(translate_libraw_error)?;
        let snapshot = snapshot(&context)?;
        map_snapshot(snapshot, revision)
    }
}

fn translate_libraw_error(error: LibrawError) -> DecodeError {
    match error {
        LibrawError::Libraw { code, message } => match code {
            LibrawErrorCode::FileUnsupported
            | LibrawErrorCode::UnspecifiedError
            | LibrawErrorCode::NotImplemented => DecodeError::Unsupported(message.into()),
            LibrawErrorCode::DataError
            | LibrawErrorCode::IoError
            | LibrawErrorCode::InputClosed => DecodeError::InvalidInput(message.into()),
            _ => DecodeError::Backend(message.into()),
        },
        other => DecodeError::Backend(other.to_string().into()),
    }
}

fn ensure_supported_path(path: &Path) -> Result<(), DecodeError> {
    if path.to_str().is_none() {
        return Err(DecodeError::InvalidInput(
            "LibRaw input path is not valid UTF-8".into(),
        ));
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| DecodeError::Unsupported("RAW file has no extension".into()))?;
    if ["dng", "arw", "nef", "cr2", "cr3"]
        .iter()
        .any(|supported| extension.eq_ignore_ascii_case(supported))
    {
        Ok(())
    } else {
        Err(DecodeError::Unsupported(
            format!("unsupported RAW extension: {extension}").into(),
        ))
    }
}

fn map_snapshot(
    mut snapshot: RawSnapshot,
    revision: Option<String>,
) -> Result<DecodedRaw, DecodeError> {
    validate_snapshot(&snapshot)?;
    let provider = provider_id();
    let decoder = DecoderProvenance::new(
        provider.clone(),
        revision.clone().map(String::into_boxed_str),
    );
    let cfa = decode_bayer_layout(&snapshot)?;
    let active_area = active_area(&snapshot)?;
    let dimensions = PixelSize::new(snapshot.raw_width, snapshot.raw_height)
        .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))?;
    let stride = NonZeroU64::new(u64::from(snapshot.raw_pitch))
        .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))?;
    let geometry = StoredGeometry::new(dimensions, stride);
    let codes = bayer_codes(snapshot.filters, snapshot.top_margin, snapshot.left_margin)?;
    let black = black_levels(&snapshot, &codes)?;
    let white = white_levels(&snapshot, &codes)?;
    let plane = stored_plane(&mut snapshot)?;
    let sample_spec = sample_spec(&snapshot, &plane)?;

    let tracked =
        |path: &str, confidence| provenance(&provider, revision.as_deref(), path, confidence);
    let camera = CameraIdentity {
        make: non_empty(snapshot.make.clone()),
        model: non_empty(snapshot.model.clone()),
        normalized_make: non_empty(snapshot.normalized_make.clone()),
        normalized_model: non_empty(snapshot.normalized_model.clone()),
    };
    let orientation = map_orientation(snapshot.flip)?;
    let default_crop = map_default_crop(snapshot.default_crop);
    let common = CommonMetadata {
        camera: Some(Sourced::new(
            camera,
            tracked("idata.make/model", Confidence::Exact),
        )),
        sample_spec: Some(Sourced::new(
            sample_spec,
            tracked("color.raw_bps", Confidence::Exact),
        )),
        stored_geometry: Some(Sourced::new(
            geometry,
            tracked("sizes.raw_width/raw_height/raw_pitch", Confidence::Exact),
        )),
        active_area: Some(Sourced::new(
            active_area,
            tracked("sizes.margins/width/height", Confidence::Exact),
        )),
        default_crop: default_crop.map(|crop| {
            Sourced::new(
                crop,
                tracked("color.dng_levels.default_crop", Confidence::Exact),
            )
        }),
        cfa: Some(Sourced::new(
            cfa,
            tracked("idata.filters + sizes.margins", Confidence::Exact),
        )),
        black: Some(Sourced::new(
            black,
            tracked("color.black + color.cblack", Confidence::Exact),
        )),
        white: Some(Sourced::new(
            white,
            tracked("color.maximum + color.linear_max", Confidence::Inferred),
        )),
        orientation: Some(Sourced::new(
            orientation,
            tracked("sizes.flip", Confidence::Exact),
        )),
        ..CommonMetadata::default()
    };
    let metadata = ValidatedMetadata::try_new(
        common,
        extensible_metadata(&snapshot, &provider, revision.as_deref())?,
        decoder,
    )
    .map_err(invalid_output)?;
    DecodedRaw::try_new(metadata, plane).map_err(invalid_output)
}

fn validate_snapshot(snapshot: &RawSnapshot) -> Result<(), DecodeError> {
    if snapshot.progress_flags & LIBRAW_POSTPROCESS_PROGRESS_MASK != 0 {
        return Err(DecodeError::Backend(
            "LibRaw input has already entered post-processing".into(),
        ));
    }
    if snapshot.raw_count != 1 {
        return Err(DecodeError::Unsupported(
            format!("multishot RAW count {} is unsupported", snapshot.raw_count).into(),
        ));
    }
    if snapshot.is_foveon {
        return Err(DecodeError::Unsupported(
            "Foveon sensors are unsupported".into(),
        ));
    }
    if snapshot.has_color3_image
        || snapshot.has_color4_image
        || snapshot.has_float3_image
        || snapshot.has_float4_image
    {
        return Err(DecodeError::Unsupported(
            "multichannel or LinearRaw payloads are unsupported".into(),
        ));
    }
    if snapshot.colors != 3 || !snapshot.cdesc.starts_with("RGBG") {
        return Err(DecodeError::Unsupported(
            format!(
                "non-RGB CFA description is unsupported: colors={}, cdesc={}",
                snapshot.colors, snapshot.cdesc
            )
            .into(),
        ));
    }
    let is_dng = snapshot.dng_version != 0;
    let make = if snapshot.normalized_make.is_empty() {
        snapshot.make.as_str()
    } else {
        snapshot.normalized_make.as_str()
    };
    let target_vendor = ["canon", "nikon", "sony"]
        .iter()
        .any(|vendor| make.eq_ignore_ascii_case(vendor));
    if !is_dng && !target_vendor {
        return Err(DecodeError::Unsupported(
            format!("LibRaw format for camera make {make:?} is outside the initial scope").into(),
        ));
    }
    if snapshot.buffer.is_none() {
        return Err(invalid_output(RawValidationError::UnsupportedNonMosaic));
    }
    Ok(())
}

fn decode_bayer_layout(snapshot: &RawSnapshot) -> Result<BayerLayout, DecodeError> {
    let codes = bayer_codes(snapshot.filters, snapshot.top_margin, snapshot.left_margin)?;
    let colors = codes.map(|code| match code {
        0 => CfaColor::Red,
        1 | 3 => CfaColor::Green,
        2 => CfaColor::Blue,
        other => CfaColor::Other(other),
    });
    let pattern =
        BayerPattern::try_from_cfa(CfaShape::BAYER_2X2, colors).map_err(invalid_output)?;
    Ok(BayerLayout::new(pattern, active_area(snapshot)?))
}

fn bayer_codes(filters: u32, top: u32, left: u32) -> Result<[u8; 4], DecodeError> {
    match filters {
        0 => return Err(invalid_output(RawValidationError::UnsupportedNonMosaic)),
        1 => {
            return Err(invalid_output(RawValidationError::UnsupportedCfa {
                rows: 16,
                columns: 16,
            }));
        }
        9 => {
            return Err(invalid_output(RawValidationError::UnsupportedCfa {
                rows: 6,
                columns: 6,
            }));
        }
        2..=8 | 10..=999 => {
            return Err(DecodeError::Unsupported(
                format!("reserved LibRaw CFA descriptor {filters}").into(),
            ));
        }
        _ => {}
    }

    let code_at = |row: u32, column: u32| {
        let index = ((row & 7) * 2 + (column & 1)) * 2;
        ((filters >> index) & 0b11) as u8
    };
    let base = [code_at(0, 0), code_at(0, 1), code_at(1, 0), code_at(1, 1)];
    for row in 0..8 {
        for column in 0..2 {
            if code_at(row, column) != base[((row & 1) * 2 + column) as usize] {
                return Err(DecodeError::Unsupported(
                    format!("LibRaw CFA is not a repeating 2x2 Bayer tile: {filters:#x}").into(),
                ));
            }
        }
    }
    Ok([
        code_at(top, left),
        code_at(top, left + 1),
        code_at(top + 1, left),
        code_at(top + 1, left + 1),
    ])
}

fn active_area(snapshot: &RawSnapshot) -> Result<ActiveArea, DecodeError> {
    let dimensions = PixelSize::new(snapshot.raw_width, snapshot.raw_height)
        .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))?;
    let bottom = snapshot
        .top_margin
        .checked_add(snapshot.height)
        .ok_or_else(|| invalid_output(RawValidationError::InvalidActiveArea))?;
    let right = snapshot
        .left_margin
        .checked_add(snapshot.width)
        .ok_or_else(|| invalid_output(RawValidationError::InvalidActiveArea))?;
    ActiveArea::try_new(
        dimensions,
        snapshot.top_margin,
        snapshot.left_margin,
        bottom,
        right,
    )
    .map_err(invalid_output)
}

fn stored_plane(snapshot: &mut RawSnapshot) -> Result<StoredPlane, DecodeError> {
    let buffer = snapshot
        .buffer
        .take()
        .ok_or_else(|| invalid_output(RawValidationError::UnsupportedNonMosaic))?;
    match buffer {
        RawBuffer::Integer(samples) => {
            if snapshot.raw_bps == 0 || snapshot.raw_bps > 16 {
                return Err(invalid_output(RawValidationError::UnsupportedSampleSpec {
                    bits_per_sample: snapshot.raw_bps.min(u32::from(u16::MAX)) as u16,
                    format: SampleFormat::UnsignedInteger,
                }));
            }
            let expected = checked_sample_count(snapshot)?;
            if samples.len() != expected {
                return Err(invalid_output(RawValidationError::InvalidGeometry));
            }
            if snapshot.raw_bps <= 8 {
                let samples = samples
                    .into_iter()
                    .map(|sample| {
                        u8::try_from(sample).map_err(|_| {
                            DecodeError::InvalidInput("LibRaw 8-bit sample exceeds u8 range".into())
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                image::ImageBuffer::from_raw(snapshot.raw_width, snapshot.raw_height, samples)
                    .map(StoredPlane::U8)
                    .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))
            } else {
                ImageBuffer::from_raw(snapshot.raw_width, snapshot.raw_height, samples)
                    .map(StoredPlane::U16)
                    .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))
            }
        }
        RawBuffer::Float(samples) => {
            if samples.len() != checked_sample_count(snapshot)? {
                return Err(invalid_output(RawValidationError::InvalidGeometry));
            }
            ImageBuffer::from_raw(snapshot.raw_width, snapshot.raw_height, samples)
                .map(StoredPlane::F32)
                .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))
        }
    }
}

fn sample_spec(snapshot: &RawSnapshot, plane: &StoredPlane) -> Result<SampleSpec, DecodeError> {
    match plane {
        StoredPlane::U8(_) | StoredPlane::U16(_) => {
            SampleSpec::try_new(snapshot.raw_bps as u8, SampleFormat::UnsignedInteger)
                .map_err(invalid_output)
        }
        StoredPlane::F32(_) => {
            let bits = if snapshot.raw_bps == 0 {
                32
            } else {
                u8::try_from(snapshot.raw_bps).map_err(|_| {
                    invalid_output(RawValidationError::UnsupportedSampleSpec {
                        bits_per_sample: u16::MAX,
                        format: SampleFormat::Float,
                    })
                })?
            };
            SampleSpec::try_new(bits, SampleFormat::Float).map_err(invalid_output)
        }
        StoredPlane::U32(_) => Err(invalid_output(RawValidationError::UnsupportedPlaneSpec)),
    }
}

fn checked_sample_count(snapshot: &RawSnapshot) -> Result<usize, DecodeError> {
    let count = u64::from(snapshot.raw_width)
        .checked_mul(u64::from(snapshot.raw_height))
        .ok_or_else(|| invalid_output(RawValidationError::InvalidGeometry))?;
    usize::try_from(count).map_err(|_| invalid_output(RawValidationError::InvalidGeometry))
}

fn black_levels(snapshot: &RawSnapshot, codes: &[u8; 4]) -> Result<BlackLevels, DecodeError> {
    if snapshot.cblack[4] != 0 || snapshot.cblack[5] != 0 {
        return Err(DecodeError::Unsupported(
            "spatial LibRaw black-level tiles are unsupported".into(),
        ));
    }
    // The snapshot is taken after `unpack` but before `raw2image` calls
    // LibRaw's `adjust_bl`, so `black` and `cblack` remain additive components.
    let common = rational(snapshot.black);
    let mut per_slot = [common; 4];
    for (slot, code) in codes.iter().enumerate() {
        per_slot[slot] = rational(snapshot.cblack[usize::from(*code)]);
    }
    BlackLevels::try_new(common, per_slot, None)
        .map_err(|error| invalid_output(RawValidationError::Metadata(error)))
}

fn white_levels(snapshot: &RawSnapshot, codes: &[u8; 4]) -> Result<WhiteLevels, DecodeError> {
    let declared = if snapshot.dng_whitelevel[0] != 0 {
        vec![rational(snapshot.dng_whitelevel[0])]
    } else {
        let linear = codes.map(|code| snapshot.linear_max[usize::from(code)]);
        if linear.iter().all(|value| *value != 0) {
            linear.map(rational).to_vec()
        } else {
            vec![rational(snapshot.maximum)]
        }
    };
    let adjusted =
        (snapshot.maximum != 0).then(|| vec![rational(snapshot.maximum); declared.len()]);
    let measured =
        (snapshot.data_maximum != 0).then(|| vec![rational(snapshot.data_maximum); declared.len()]);
    WhiteLevels::try_new(declared, adjusted, measured)
        .map_err(|error| invalid_output(RawValidationError::Metadata(error)))
}

fn map_orientation(flip: i32) -> Result<Orientation, DecodeError> {
    let value = match flip {
        0 => 1,
        3 => 3,
        5 => 8,
        6 => 6,
        other => {
            return Err(DecodeError::Unsupported(
                format!("LibRaw orientation code {other} is unsupported").into(),
            ));
        }
    };
    Orientation::new(value)
        .ok_or_else(|| DecodeError::Backend("invalid TIFF orientation mapping".into()))
}

fn map_default_crop(crop: [u16; 4]) -> Option<DefaultCrop> {
    let [origin_x, origin_y, width, height] = crop;
    if width == 0 || height == 0 {
        return None;
    }
    DefaultCrop::try_new(
        rational(u32::from(origin_x)),
        rational(u32::from(origin_y)),
        rational(u32::from(width)),
        rational(u32::from(height)),
    )
    .ok()
}

fn extensible_metadata(
    snapshot: &RawSnapshot,
    provider: &ProviderId,
    revision: Option<&str>,
) -> Result<ExtensibleMetadata, DecodeError> {
    let mut metadata = ExtensibleMetadata::default();
    insert_u64(
        &mut metadata,
        "decoder.libraw.raw_count",
        u64::from(snapshot.raw_count),
        provider,
        revision,
    )?;
    insert_u64(
        &mut metadata,
        "decoder.libraw.filters",
        u64::from(snapshot.filters),
        provider,
        revision,
    )?;
    insert_u64(
        &mut metadata,
        "decoder.libraw.progress_flags",
        u64::from(snapshot.progress_flags),
        provider,
        revision,
    )?;
    if let Some(function) = &snapshot.unpack_function {
        metadata
            .insert(MetadataEntry {
                key: MetadataKey::try_new("decoder.libraw.unpack_function")
                    .map_err(|error| invalid_output(RawValidationError::Metadata(error)))?,
                value: MetadataValue::Text(function.clone().into_boxed_str()),
                unit: None,
                coordinate_space: None,
                provenance: provenance(
                    provider,
                    revision,
                    "libraw_unpack_function_name",
                    Confidence::Exact,
                ),
                original: None,
            })
            .map_err(|error| invalid_output(RawValidationError::Metadata(error)))?;
    }
    Ok(metadata)
}

fn insert_u64(
    metadata: &mut ExtensibleMetadata,
    key: &str,
    value: u64,
    provider: &ProviderId,
    revision: Option<&str>,
) -> Result<(), DecodeError> {
    metadata
        .insert(MetadataEntry {
            key: MetadataKey::try_new(key)
                .map_err(|error| invalid_output(RawValidationError::Metadata(error)))?,
            value: MetadataValue::U64(value),
            unit: Some(MetadataUnit::Dimensionless),
            coordinate_space: Some(CoordinateSpace::None),
            provenance: provenance(provider, revision, key, Confidence::Exact),
            original: None,
        })
        .map_err(|error| invalid_output(RawValidationError::Metadata(error)))
}

fn provenance(
    provider: &ProviderId,
    revision: Option<&str>,
    path: &str,
    confidence: Confidence,
) -> Provenance {
    Provenance::new(
        provider.clone(),
        SourceKind::Decoder {
            backend: "libraw".into(),
            revision: revision.map(Into::into),
            path: path.into(),
        },
        AuthorityClass::DecoderObservation,
        confidence,
    )
}

fn provider_id() -> ProviderId { ProviderId::try_new("libraw").expect("static provider is valid") }

fn rational(value: u32) -> Rational { Rational::new(i64::from(value), NonZeroU32::MIN) }

fn non_empty(value: String) -> Option<Box<str>> {
    (!value.is_empty()).then(|| value.into_boxed_str())
}

fn invalid_output(error: RawValidationError) -> DecodeError { DecodeError::InvalidOutput(error) }

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_filters(tile: [u8; 4]) -> u32 {
        let mut filters = 0_u32;
        for row in 0..8 {
            for column in 0..2 {
                let index = (row * 2 + column) * 2;
                filters |= u32::from(tile[((row & 1) * 2 + column) as usize]) << index;
            }
        }
        filters
    }

    fn snapshot(tile: [u8; 4]) -> RawSnapshot {
        RawSnapshot {
            raw_height: 2,
            raw_width: 2,
            raw_pitch: 4,
            height: 2,
            width: 2,
            top_margin: 0,
            left_margin: 0,
            flip: 0,
            make: "Canon".into(),
            model: "Test".into(),
            normalized_make: "Canon".into(),
            normalized_model: "Test".into(),
            cdesc: "RGBG".into(),
            raw_count: 1,
            is_foveon: false,
            dng_version: 0,
            colors: 3,
            filters: encode_filters(tile),
            raw_bps: 14,
            black: 64,
            cblack: [0, 1, 2, 3, 0, 0],
            maximum: 16_383,
            data_maximum: 0,
            linear_max: [16_000, 16_001, 16_002, 16_003],
            dng_whitelevel: [0; 4],
            default_crop: [0, 0, 2, 2],
            has_color3_image: false,
            has_color4_image: false,
            has_float3_image: false,
            has_float4_image: false,
            progress_flags: LIBRAW_RAW2_IMAGE_PROGRESS >> 1,
            unpack_function: Some("canon_load_raw".into()),
            buffer: Some(RawBuffer::Integer(vec![100, 200, 300, 400])),
        }
    }

    #[test]
    fn every_bayer_phase_is_decoded_at_the_active_origin() {
        let cases = [
            ([0, 1, 3, 2], BayerPattern::Rggb, [0, 1, 3, 2]),
            ([2, 3, 1, 0], BayerPattern::Bggr, [2, 3, 1, 0]),
            ([1, 0, 2, 3], BayerPattern::Grbg, [1, 0, 2, 3]),
            ([3, 2, 0, 1], BayerPattern::Gbrg, [3, 2, 0, 1]),
        ];
        for (tile, pattern, origin_codes) in cases {
            let filters = encode_filters(tile);
            assert_eq!(bayer_codes(filters, 0, 0).unwrap(), origin_codes);
            let mut input = snapshot(tile);
            input.filters = filters;
            assert_eq!(decode_bayer_layout(&input).unwrap().pattern(), pattern);
        }
    }

    #[test]
    fn active_origin_rebases_the_bayer_phase() {
        let filters = encode_filters([0, 1, 3, 2]);
        assert_eq!(bayer_codes(filters, 1, 1).unwrap(), [2, 3, 1, 0]);
    }

    #[test]
    fn native_samples_and_metadata_are_preserved() {
        let decoded = map_snapshot(snapshot([0, 1, 3, 2]), Some("0.22.2".into())).unwrap();
        assert_eq!(decoded.metadata().cfa().pattern(), BayerPattern::Rggb);
        assert_eq!(
            decoded.metadata().stored_geometry().dimensions(),
            PixelSize::new(2, 2).unwrap()
        );
        assert_eq!(
            decoded
                .metadata()
                .common()
                .orientation
                .as_ref()
                .unwrap()
                .value
                .value(),
            1
        );
        assert_eq!(decoded.metadata().decoder().revision(), Some("0.22.2"));
        assert_eq!(decoded.metadata().black().per_slot()[0], rational(0));
        assert_eq!(decoded.metadata().black().per_slot()[1], rational(1));
        assert_eq!(decoded.metadata().black().per_slot()[3], rational(2));
        let StoredPlane::U16(plane) = decoded.plane() else {
            panic!("14-bit input uses a u16 plane");
        };
        assert_eq!(plane.as_raw(), &[100, 200, 300, 400]);
    }

    #[test]
    fn dng_declared_white_level_wins() {
        let mut input = snapshot([0, 1, 3, 2]);
        input.dng_version = 0x01_04_00_00;
        input.dng_whitelevel = [12_000, 0, 0, 0];
        let decoded = map_snapshot(input, None).unwrap();
        assert_eq!(decoded.metadata().white().declared(), &[rational(12_000)]);
    }

    #[test]
    fn native_storage_variants_follow_sample_precision() {
        let mut eight_bit = snapshot([0, 1, 3, 2]);
        eight_bit.raw_bps = 8;
        eight_bit.maximum = 255;
        eight_bit.cblack = [0, 1, 2, 3, 0, 0];
        eight_bit.black = 8;
        eight_bit.linear_max = [240, 241, 242, 243];
        eight_bit.buffer = Some(RawBuffer::Integer(vec![20, 30, 40, 50]));
        let decoded = map_snapshot(eight_bit, None).unwrap();
        assert!(matches!(decoded.plane(), StoredPlane::U8(_)));
        assert_eq!(
            decoded.sample_spec(),
            SampleSpec::try_new(8, SampleFormat::UnsignedInteger).unwrap()
        );

        let mut float = snapshot([0, 1, 3, 2]);
        float.raw_bps = 0;
        float.maximum = 1;
        float.black = 0;
        float.cblack = [0; 6];
        float.linear_max = [0; 4];
        float.buffer = Some(RawBuffer::Float(vec![0.1, 0.2, 0.3, 0.4]));
        let decoded = map_snapshot(float, None).unwrap();
        assert!(matches!(decoded.plane(), StoredPlane::F32(_)));
        assert_eq!(
            decoded.sample_spec(),
            SampleSpec::try_new(32, SampleFormat::Float).unwrap()
        );
    }

    #[test]
    fn orientation_and_default_crop_are_mapped_without_rotation() {
        assert_eq!(map_orientation(0).unwrap().value(), 1);
        assert_eq!(map_orientation(3).unwrap().value(), 3);
        assert_eq!(map_orientation(5).unwrap().value(), 8);
        assert_eq!(map_orientation(6).unwrap().value(), 6);
        let crop = map_default_crop([2, 3, 100, 200]).unwrap();
        assert_eq!(crop.origin_x(), rational(2));
        assert_eq!(crop.origin_y(), rational(3));
        assert_eq!(crop.width(), rational(100));
        assert_eq!(crop.height(), rational(200));
    }

    #[test]
    fn unsupported_layouts_are_structured() {
        let mut monochrome = snapshot([0, 1, 3, 2]);
        monochrome.filters = 0;
        assert!(matches!(
            map_snapshot(monochrome, None),
            Err(DecodeError::InvalidOutput(
                RawValidationError::UnsupportedNonMosaic
            ))
        ));

        let mut xtrans = snapshot([0, 1, 3, 2]);
        xtrans.filters = 9;
        assert!(matches!(
            map_snapshot(xtrans, None),
            Err(DecodeError::InvalidOutput(
                RawValidationError::UnsupportedCfa {
                    rows: 6,
                    columns: 6
                }
            ))
        ));

        let mut foveon = snapshot([0, 1, 3, 2]);
        foveon.is_foveon = true;
        assert!(matches!(
            map_snapshot(foveon, None),
            Err(DecodeError::Unsupported(_))
        ));
    }

    #[test]
    fn spatial_black_and_non_bayer_filters_are_rejected() {
        let mut spatial = snapshot([0, 1, 3, 2]);
        spatial.cblack[4] = 2;
        spatial.cblack[5] = 2;
        assert!(matches!(
            map_snapshot(spatial, None),
            Err(DecodeError::Unsupported(_))
        ));

        let mut quad = snapshot([0, 1, 3, 2]);
        quad.filters = encode_filters([0, 1, 3, 2]) ^ 0b11;
        assert!(matches!(
            map_snapshot(quad, None),
            Err(DecodeError::Unsupported(_))
        ));
    }

    #[test]
    fn non_target_vendor_and_progress_state_are_rejected() {
        let mut deferred = snapshot([0, 1, 3, 2]);
        deferred.make = "FUJIFILM".into();
        deferred.normalized_make = "Fujifilm".into();
        assert!(matches!(
            map_snapshot(deferred, None),
            Err(DecodeError::Unsupported(_))
        ));

        let mut processed = snapshot([0, 1, 3, 2]);
        processed.progress_flags |= LIBRAW_RAW2_IMAGE_PROGRESS;
        assert!(matches!(
            map_snapshot(processed, None),
            Err(DecodeError::Backend(_))
        ));
    }
}
