use fornax_dng::metadata::DngTag;
use fornax_hasselblad::HasselbladImage;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, Value, unsigned_vec};

const RAW_IFD_TAG: Tag = Tag(330);

#[test]
fn selects_the_non_reduced_raw_subifd_and_decodes_little_endian_words() {
    let source = synthetic_3fr(
        "Hasselblad H3D",
        "Hasselblad H3D",
        4,
        2,
        1,
        &[1_u16, 2, 3, 4, 5, 6, 7, 8]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
        Some(([0, 0], [4, 2])),
        None,
        false,
    );

    let image = HasselbladImage::parse(&source).unwrap();
    let RawMosaic::U16(mosaic) = image.raw_mosaic().unwrap() else {
        panic!("3FR words must decode as u16");
    };

    assert_eq!(mosaic.dimensions(), (4, 2));
    assert_eq!(mosaic.as_raw().as_slice(), &[1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn rejects_uncompressed_payloads_with_the_wrong_byte_count() {
    let source = synthetic_3fr(
        "Hasselblad H3D",
        "Hasselblad H3D",
        4,
        2,
        1,
        &[0; 8],
        Some(([0, 0], [4, 2])),
        None,
        false,
    );

    let error = HasselbladImage::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::InvalidInput(_)));
}

#[test]
fn compression_seven_uses_the_payload_through_eof() {
    let source = synthetic_3fr(
        "Hasselblad H3D",
        "Hasselblad H3D",
        4,
        2,
        7,
        &[0xff, 0xd9],
        Some(([0, 0], [4, 2])),
        Some(u32::MAX),
        false,
    );
    let image = HasselbladImage::parse(&source).unwrap();

    let error = image.raw_mosaic().unwrap_err();

    assert!(
        error.to_string().contains("Hasselblad lossless JPEG"),
        "unexpected error: {error}"
    );
}

#[test]
fn x1d_active_borders_rebase_the_default_crop() {
    let source = synthetic_3fr(
        "Hasselblad X1D",
        "Hasselblad X1D",
        120,
        120,
        1,
        &vec![0; 120 * 120 * 2],
        Some(([50, 100], [8, 20])),
        None,
        false,
    );
    let metadata = HasselbladImage::parse(&source)
        .unwrap()
        .dng_metadata()
        .unwrap();

    assert_eq!(
        unsigned_vec::<_, u32>(&metadata, DngTag::ActiveArea).unwrap(),
        [96, 46, 120, 62]
    );
    assert_eq!(
        unsigned_vec::<_, u32>(&metadata, DngTag::DefaultCropOrigin).unwrap(),
        [4, 4]
    );
    assert_eq!(
        unsigned_vec::<_, u32>(&metadata, DngTag::DefaultCropSize).unwrap(),
        [8, 20]
    );
}

#[test]
fn unknown_tagless_models_are_unsupported() {
    let source = synthetic_3fr(
        "Unknown Camera",
        "Unknown Camera",
        4,
        2,
        1,
        &[0; 16],
        Some(([0, 0], [4, 2])),
        None,
        false,
    );

    let error = HasselbladImage::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn source_cfa_metadata_allows_raw_export_but_rejects_pipeline_handoff() {
    let source = synthetic_3fr(
        "Unknown Camera",
        "Unknown Camera",
        2,
        2,
        1,
        &[0; 8],
        Some(([0, 0], [2, 2])),
        None,
        true,
    );
    let image = HasselbladImage::parse(&source).unwrap();

    assert_eq!(
        unsigned_vec::<_, u32>(&image.dng_metadata().unwrap(), TiffTag::CfaRepeatPatternDim,)
            .unwrap(),
        [2, 1]
    );
    assert!(matches!(
        image.process_metadata(),
        Err(DecodeError::Unsupported(_))
    ));
}

#[allow(clippy::too_many_arguments)]
fn synthetic_3fr(
    unique_model: &str,
    model: &str,
    width: u32,
    height: u32,
    compression: u16,
    payload: &[u8],
    crop: Option<([u32; 2], [u32; 2])>,
    strip_byte_count: Option<u32>,
    source_cfa: bool,
) -> Vec<u8> {
    let (crop_origin, crop_size) = crop.unwrap_or(([0, 0], [width, height]));
    let mut raw = vec![
        (Tag::from(TiffTag::NewSubfileType), Value::Unsigned(0)),
        (Tag::from(TiffTag::ImageWidth), Value::Unsigned(width)),
        (Tag::from(TiffTag::ImageLength), Value::Unsigned(height)),
        (Tag::from(TiffTag::BitsPerSample), Value::Short(16)),
        (Tag::from(TiffTag::Compression), Value::Short(compression)),
        (
            Tag::from(TiffTag::PhotometricInterpretation),
            Value::Short(32_803),
        ),
        (Tag::from(TiffTag::SamplesPerPixel), Value::Short(1)),
        (Tag::from(TiffTag::RowsPerStrip), Value::Unsigned(height)),
        (Tag::from(DngTag::WhiteLevel), Value::Short(u16::MAX)),
        (
            Tag::from(DngTag::DefaultCropOrigin),
            Value::List(crop_origin.into_iter().map(Value::Unsigned).collect()),
        ),
        (
            Tag::from(DngTag::DefaultCropSize),
            Value::List(crop_size.into_iter().map(Value::Unsigned).collect()),
        ),
    ];
    if source_cfa {
        raw.push((
            Tag::from(TiffTag::CfaRepeatPatternDim),
            Value::List(vec![Value::Unsigned(2), Value::Unsigned(1)]),
        ));
        raw.push((
            Tag::from(TiffTag::CfaPattern),
            Value::List(vec![Value::Byte(0), Value::Byte(1)]),
        ));
    }
    build_tiff(
        vec![
            (Tag::from(TiffTag::NewSubfileType), Value::Unsigned(1)),
            (Tag::from(TiffTag::Make), Value::Ascii("Hasselblad".into())),
            (Tag::from(TiffTag::Model), Value::Ascii(model.into())),
            (
                Tag::from(DngTag::UniqueCameraModel),
                Value::Ascii(unique_model.into()),
            ),
        ],
        raw,
        payload,
        strip_byte_count,
    )
}

fn build_tiff(
    root: Vec<(impl Into<Tag>, Value)>,
    raw: Vec<(impl Into<Tag>, Value)>,
    payload: &[u8],
    declared_byte_count: Option<u32>,
) -> Vec<u8> {
    let mut root = root
        .into_iter()
        .map(|(tag, value)| (tag.into(), value))
        .collect::<Vec<_>>();
    let mut raw = raw
        .into_iter()
        .map(|(tag, value)| (tag.into(), value))
        .collect::<Vec<_>>();
    let byte_count = declared_byte_count.unwrap_or(u32::try_from(payload.len()).unwrap());
    raw.push((TiffTag::StripOffsets.into(), Value::Unsigned(0)));
    raw.push((TiffTag::StripByteCounts.into(), Value::Unsigned(byte_count)));
    let raw_ifd_tag = RAW_IFD_TAG;
    root.push((raw_ifd_tag, Value::Unsigned(0)));

    let mut bytes = vec![0; 8];
    let (root_table_end, root_fields) = write_ifd(&mut bytes, 8, &root);
    let payload_offset = u32::try_from(root_table_end).unwrap();
    bytes.extend_from_slice(payload);
    let raw_offset = u32::try_from(bytes.len()).unwrap();
    let raw_ifd_offset = bytes.len();
    write_ifd(&mut bytes, raw_ifd_offset, &raw);

    for (tag, field_position) in root_fields {
        if tag == raw_ifd_tag {
            bytes[field_position..field_position + 4].copy_from_slice(&raw_offset.to_le_bytes());
        }
    }
    let raw_entries = raw_entries(&mut bytes, raw_offset);
    for (tag, field_position) in raw_entries {
        if tag == Tag::from(TiffTag::StripOffsets) {
            bytes[field_position..field_position + 4]
                .copy_from_slice(&payload_offset.to_le_bytes());
        }
    }

    bytes[..2].copy_from_slice(b"II");
    bytes[2..4].copy_from_slice(&42_u16.to_le_bytes());
    bytes[4..8].copy_from_slice(&8_u32.to_le_bytes());
    bytes
}

fn write_ifd(
    bytes: &mut Vec<u8>,
    offset: usize,
    entries: &[(Tag, Value)],
) -> (usize, Vec<(Tag, usize)>) {
    assert_eq!(bytes.len(), offset);
    let mut sorted = entries.to_vec();
    sorted.sort_unstable_by_key(|(tag, _)| *tag);
    bytes.extend_from_slice(&u16::try_from(sorted.len()).unwrap().to_le_bytes());
    let table_start = bytes.len();
    bytes.resize(table_start + sorted.len() * 12 + 4, 0);
    let mut fields = Vec::new();
    for (index, (tag, value)) in sorted.into_iter().enumerate() {
        let field_position = table_start + index * 12;
        let (field_type, count, encoded) = encode_value(&value);
        bytes[field_position..field_position + 2].copy_from_slice(&tag.0.to_le_bytes());
        bytes[field_position + 2..field_position + 4].copy_from_slice(&field_type.to_le_bytes());
        bytes[field_position + 4..field_position + 8].copy_from_slice(&count.to_le_bytes());
        if encoded.len() <= 4 {
            bytes[field_position + 8..field_position + 8 + encoded.len()].copy_from_slice(&encoded);
        } else {
            let value_offset = bytes.len();
            bytes.extend_from_slice(&encoded);
            bytes[field_position + 8..field_position + 12]
                .copy_from_slice(&u32::try_from(value_offset).unwrap().to_le_bytes());
        }
        fields.push((tag, field_position + 8));
    }
    (bytes.len(), fields)
}

fn raw_entries(bytes: &mut [u8], raw_offset: u32) -> Vec<(Tag, usize)> {
    let offset = usize::try_from(raw_offset).unwrap();
    let count = usize::from(u16::from_le_bytes(
        bytes[offset..offset + 2].try_into().unwrap(),
    ));
    (0..count)
        .map(|index| {
            let field = offset + 2 + index * 12;
            let tag = Tag(u16::from_le_bytes(
                bytes[field..field + 2].try_into().unwrap(),
            ));
            (tag, field + 8)
        })
        .collect()
}

fn encode_value(value: &Value) -> (u16, u32, Vec<u8>) {
    if let Value::List(values) = value {
        let mut encoded = Vec::new();
        let mut field_type = 0;
        for value in values {
            let (current_type, count, current) = encode_value(value);
            assert_eq!(count, 1);
            assert!(field_type == 0 || field_type == current_type);
            field_type = current_type;
            encoded.extend_from_slice(&current);
        }
        return (field_type, u32::try_from(values.len()).unwrap(), encoded);
    }
    match value {
        Value::Byte(value) => (1, 1, vec![*value]),
        Value::Ascii(value) => {
            let mut encoded = value.as_bytes().to_vec();
            encoded.push(0);
            (2, u32::try_from(encoded.len()).unwrap(), encoded)
        }
        Value::Short(value) => (3, 1, value.to_le_bytes().to_vec()),
        Value::Unsigned(value) => (4, 1, value.to_le_bytes().to_vec()),
        Value::Rational(numerator, denominator) => {
            let mut encoded = numerator.to_le_bytes().to_vec();
            encoded.extend_from_slice(&denominator.to_le_bytes());
            (5, 1, encoded)
        }
        Value::SRational(numerator, denominator) => {
            let mut encoded = numerator.to_le_bytes().to_vec();
            encoded.extend_from_slice(&denominator.to_le_bytes());
            (10, 1, encoded)
        }
        _ => panic!("unsupported synthetic TIFF value {value:?}"),
    }
}
