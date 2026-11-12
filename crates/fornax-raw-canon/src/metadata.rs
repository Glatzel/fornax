use fornax_dng::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, Tag, Value};
use image::math::Rect;

use crate::CanonCrxCfa;
use crate::color_data::{ColorData, ColorDataStatus};
use crate::container::Cmt;

const CMT3_ALLOWLIST: &[u16] = &[
    0x0001, 0x000c, 0x0010, 0x001e, 0x0028, 0x0095, 0x0096, 0x00b4, 0x4001,
];

pub fn map(
    cmt: &Cmt,
    width: u32,
    height: u32,
    active: Rect,
    crop: Rect,
    cfa: CanonCrxCfa,
    color_data: &ColorDataStatus,
    bit_depth: u8,
    quality_mode: Option<u16>,
) -> Result<Directory, DecodeError> {
    let mut directory = Directory::default();
    copy_all(&cmt.cmt1, &mut directory);
    copy_all(&cmt.cmt2, &mut directory);
    copy_all(&cmt.cmt4, &mut directory);
    for &tag in CMT3_ALLOWLIST {
        if let Some(value) = cmt.cmt3.get(&Tag(tag)) {
            directory.insert(Tag(tag), value.clone());
        }
    }
    directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(width));
    directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(height));
    directory.insert(
        TiffTag::BitsPerSample.into(),
        Value::Short(u16::from(bit_depth)),
    );
    directory.insert(
        TiffTag::CfaRepeatPatternDim.into(),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    directory.insert(
        DngTag::CfaPlaneColor.into(),
        Value::List(vec![Value::Short(0), Value::Short(1), Value::Short(2)]),
    );
    directory.insert(DngTag::CfaLayout.into(), Value::Short(1));
    let pattern = shifted_pattern(cfa, active.x, active.y);
    directory.insert(
        TiffTag::CfaPattern.into(),
        Value::List(pattern.into_iter().map(Value::Byte).collect()),
    );
    directory.insert(
        DngTag::ActiveArea.into(),
        Value::List(vec![
            Value::Unsigned(active.y),
            Value::Unsigned(active.x),
            Value::Unsigned(active.y + active.height),
            Value::Unsigned(active.x + active.width),
        ]),
    );
    directory.insert(
        DngTag::DefaultCropOrigin.into(),
        Value::List(vec![
            Value::Unsigned(crop.x.saturating_sub(active.x)),
            Value::Unsigned(crop.y.saturating_sub(active.y)),
        ]),
    );
    directory.insert(
        DngTag::DefaultCropSize.into(),
        Value::List(vec![
            Value::Unsigned(crop.width),
            Value::Unsigned(crop.height),
        ]),
    );
    if let Some(color_data) = color_data.parsed() {
        add_color_data(&mut directory, color_data, quality_mode)?;
    }
    Ok(directory)
}

fn add_color_data(
    directory: &mut Directory,
    color_data: &ColorData,
    quality_mode: Option<u16>,
) -> Result<(), DecodeError> {
    directory.insert(
        DngTag::BlackLevelRepeatDim.into(),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    directory.insert(
        DngTag::BlackLevel.into(),
        Value::List(
            color_data
                .black_level
                .into_iter()
                .map(Value::Short)
                .collect(),
        ),
    );
    if color_data.specular_white_level != 0 {
        directory.insert(
            DngTag::WhiteLevel.into(),
            Value::Unsigned(u32::from(color_data.specular_white_level)),
        );
    }
    let neutral = if quality_mode == Some(130) {
        [1_u16, 1, 1]
    } else {
        let green = u32::midpoint(
            u32::from(color_data.white_balance[1]),
            u32::from(color_data.white_balance[2]),
        );
        if green == 0 {
            return Err(crate::invalid("Canon ColorData has zero green balance"));
        }
        [
            color_data.white_balance[0],
            u16::try_from(green).unwrap_or(u16::MAX),
            color_data.white_balance[3],
        ]
    };
    let denominator = u32::from(neutral[1]).max(1);
    directory.insert(
        DngTag::AsShotNeutral.into(),
        Value::List(
            neutral
                .into_iter()
                .map(|value| Value::Rational(u32::from(value), denominator))
                .collect(),
        ),
    );
    Ok(())
}

pub fn shifted_pattern(cfa: CanonCrxCfa, x: u32, y: u32) -> [u8; 4] {
    let base = cfa.pattern();
    let x_shift = usize::try_from(x & 1).unwrap_or(0);
    let y_shift = usize::try_from(y & 1).unwrap_or(0);
    [
        base[y_shift * 2 + x_shift],
        base[y_shift * 2 + (x_shift + 1) % 2],
        base[((y_shift + 1) % 2) * 2 + x_shift],
        base[((y_shift + 1) % 2) * 2 + (x_shift + 1) % 2],
    ]
}

fn copy_all(source: &Directory, target: &mut Directory) {
    for (tag, value) in source {
        target.insert(*tag, value.clone());
    }
}

impl ColorDataStatus {
    pub const fn parsed(&self) -> Option<&ColorData> {
        match self {
            Self::Parsed(color_data) => Some(color_data),
            Self::Missing | Self::Unknown(_) => None,
        }
    }
}
