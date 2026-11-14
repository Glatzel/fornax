use fornax_bmff::{Bmff, BmffError, BmffLimits, BoxRef, FourCc, SampleTable, SampleTableLimits};
use fornax_raw_core::error::DecodeError;
use fornax_tiff::Directory;
use image::math::Rect;

use crate::color_data::{self, ColorDataStatus};
use crate::{CanonCrxCfa, CanonCrxHeader, invalid, metadata};

const CR3_DESCRIPTION_UUID: [u8; 16] = [
    0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a, 0x48,
];
const CRX_BRAND: FourCc = FourCc::new(*b"crx ");

#[derive(Clone, Copy, Debug)]
pub struct SampleRange {
    pub offset: usize,
    pub size: usize,
}

#[derive(Clone, Debug)]
pub struct CodecHeader {
    pub header: CanonCrxHeader,
    pub cfa: CanonCrxCfa,
    pub width: usize,
    pub height: usize,
    pub bit_depth: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub active: Rect,
    pub crop: Rect,
}

pub struct Parsed {
    pub sample: SampleRange,
    pub header: CodecHeader,
    pub metadata: Directory,
    pub color_data: ColorDataStatus,
    pub quality_mode: Option<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CctpRecord {
    image_type: u64,
    track_index: u32,
}

pub fn parse(source: &[u8], sample_index: usize) -> Result<Parsed, DecodeError> {
    let bmff = Bmff::parse(source).map_err(bmff_invalid)?;
    validate_brand(&bmff)?;
    let moov = top_child(&bmff, FourCc::new(*b"moov"))?
        .ok_or_else(|| invalid("Canon CR3 moov box is missing"))?;
    let cr3desc = moov
        .children(BmffLimits::default())
        .map_err(bmff_invalid)?
        .into_iter()
        .find(|item| item.uuid() == Some(CR3_DESCRIPTION_UUID))
        .ok_or_else(|| invalid("Canon CR3 description box is missing"))?;
    let cctp = child(&cr3desc, FourCc::new(*b"CCTP"), 0)?;
    let records = cctp.map(parse_cctp).transpose()?.unwrap_or_default();
    let tracks = parse_tracks(&moov)?;
    let raw_index = select_raw_track(&records, &tracks)?;
    let raw_track = tracks
        .get(raw_index)
        .ok_or_else(|| invalid("Canon CR3 selected track is missing"))?;
    let sample = raw_track
        .sample_table
        .sample(sample_index, source.len())
        .map_err(bmff_invalid)?;
    let sample_range = SampleRange {
        offset: sample.offset,
        size: sample.size,
    };
    let craw = child(&raw_track.stsd, FourCc::new(*b"CRAW"), 8)?.ok_or_else(|| {
        DecodeError::Unsupported("Canon CR3 track has no raw sample entry".into())
    })?;
    let cmp1 = child(&craw, FourCc::new(*b"CMP1"), 82)?
        .ok_or_else(|| invalid("Canon CR3 CMP1 box is missing"))?;
    let cmp1 = parse_cmp1(cmp1)?;
    let cfa = CanonCrxCfa::from_cmp1_layout(cmp1.cfa_layout)
        .map_err(|error| DecodeError::Unsupported(error.to_string().into()))?;
    let header = CanonCrxHeader::new(
        cmp1.version,
        cmp1.width,
        cmp1.height,
        cmp1.tile_width,
        cmp1.tile_height,
        cmp1.bit_depth,
        cmp1.encoding_type,
        cmp1.levels,
        cmp1.mdat_header_size,
        cmp1.median_precision,
    )
    .map_err(|error| match error {
        fornax_codec::CodecError::Unsupported(message) => DecodeError::Unsupported(message),
        fornax_codec::CodecError::InvalidInput(message) => DecodeError::InvalidInput(message),
    })?;
    let iad1 = match child(&craw, FourCc::new(*b"CDI1"), 82)? {
        Some(cdi1) => child(&cdi1, FourCc::new(*b"IAD1"), 4)?,
        None => None,
    };
    let geometry = parse_geometry(iad1, cmp1.width, cmp1.height)?;
    let cmt = parse_cmt(&cr3desc)?;
    let quality_mode = cmt
        .cmt3
        .get(&fornax_tiff::Tag(0x0001))
        .map(|value| u16::try_from(value.force_u32(3)).unwrap_or(u16::MAX));
    let color_data = parse_color_data(source, &tracks)?;
    let metadata = metadata::map(
        &cmt,
        cmp1.width,
        cmp1.height,
        geometry.active,
        geometry.crop,
        cfa,
        &color_data,
        cmp1.bit_depth,
        quality_mode,
    )?;
    Ok(Parsed {
        sample: sample_range,
        header: CodecHeader {
            header,
            cfa,
            width: usize::try_from(cmp1.width)
                .map_err(|_| invalid("Canon CR3 width exceeds usize"))?,
            height: usize::try_from(cmp1.height)
                .map_err(|_| invalid("Canon CR3 height exceeds usize"))?,
            bit_depth: cmp1.bit_depth,
        },
        metadata,
        color_data,
        quality_mode,
    })
}

#[derive(Clone, Debug)]
struct Track<'a> {
    stsd: BoxRef<'a>,
    sample_table: SampleTable,
    kind: TrackKind,
}

// This marker lifetime is replaced by the parser's borrowed source below.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrackKind {
    Craw,
    Ctmd,
    Other,
}

fn parse_tracks<'a>(moov: &BoxRef<'a>) -> Result<Vec<Track<'a>>, DecodeError> {
    let mut tracks = Vec::new();
    for trak in moov
        .children(BmffLimits::default())
        .map_err(bmff_invalid)?
        .into_iter()
        .filter(|item| item.typ() == FourCc::new(*b"trak"))
    {
        let Some(mdia) = child(&trak, FourCc::new(*b"mdia"), 0)? else {
            continue;
        };
        let Some(minf) = child(&mdia, FourCc::new(*b"minf"), 0)? else {
            continue;
        };
        let Some(stbl) = child(&minf, FourCc::new(*b"stbl"), 0)? else {
            continue;
        };
        let Some(stsd) = child(&stbl, FourCc::new(*b"stsd"), 0)? else {
            continue;
        };
        let stsc = child(&stbl, FourCc::new(*b"stsc"), 0)?
            .ok_or_else(|| invalid("Canon CR3 stsc box is missing"))?;
        let stsz = child(&stbl, FourCc::new(*b"stsz"), 0)?
            .ok_or_else(|| invalid("Canon CR3 stsz box is missing"))?;
        let chunk_offsets = child(&stbl, FourCc::new(*b"stco"), 0)?
            .or_else(|| child(&stbl, FourCc::new(*b"co64"), 0).ok().flatten());
        let Some(chunk_offsets) = chunk_offsets else {
            return Err(invalid("Canon CR3 chunk-offset box is missing"));
        };
        let chunk_type = if chunk_offsets.typ() == FourCc::new(*b"co64") {
            *b"co64"
        } else {
            *b"stco"
        };
        let sample_table = SampleTable::parse(
            stsc.payload(),
            stsz.payload(),
            chunk_type,
            chunk_offsets.payload(),
            SampleTableLimits::default(),
        )
        .map_err(bmff_invalid)?;
        let kind = if child(&stsd, FourCc::new(*b"CRAW"), 8)?.is_some() {
            TrackKind::Craw
        } else if child(&stsd, FourCc::new(*b"CTMD"), 8)?.is_some() {
            TrackKind::Ctmd
        } else {
            TrackKind::Other
        };
        tracks.push(Track {
            stsd,
            sample_table,
            kind,
        });
    }
    Ok(tracks)
}

fn select_raw_track(records: &[CctpRecord], tracks: &[Track<'_>]) -> Result<usize, DecodeError> {
    let type_zero: Vec<_> = records
        .iter()
        .filter(|record| record.image_type == 0)
        .collect();
    if type_zero.len() > 1 {
        return Err(DecodeError::Unsupported(
            "Canon CR3 has ambiguous CCTP raw tracks".into(),
        ));
    }
    if let Some(record) = type_zero.first() {
        let index = usize::try_from(record.track_index)
            .ok()
            .and_then(|index| index.checked_sub(1))
            .ok_or_else(|| invalid("Canon CR3 CCTP raw track index is zero"))?;
        let track = tracks
            .get(index)
            .ok_or_else(|| invalid("Canon CR3 CCTP raw track is missing"))?;
        if track.kind != TrackKind::Craw {
            return Err(DecodeError::Unsupported(
                "Canon CR3 CCTP image type zero does not name a CRAW track".into(),
            ));
        }
        return Ok(index);
    }
    let candidates: Vec<_> = tracks
        .iter()
        .enumerate()
        .filter_map(|(index, track)| (track.kind == TrackKind::Craw).then_some(index))
        .collect();
    match candidates.as_slice() {
        [index] => Ok(*index),
        [] => Err(DecodeError::Unsupported(
            "Canon CR3 has no CRAW track".into(),
        )),
        _ => Err(DecodeError::Unsupported(
            "Canon CR3 has multiple CRAW tracks without a CCTP selector".into(),
        )),
    }
}

fn parse_cctp(cctp: BoxRef<'_>) -> Result<Vec<CctpRecord>, DecodeError> {
    let mut records = Vec::new();
    let payload = cctp
        .payload()
        .get(12..)
        .ok_or_else(|| invalid("Canon CR3 CCTP payload is truncated"))?;
    let nested = Bmff::parse(payload).map_err(bmff_invalid)?;
    for child in nested.boxes() {
        if child.typ() != FourCc::new(*b"CCDT") {
            continue;
        }
        let payload = child.payload();
        if payload.len() < 16 {
            return Err(invalid("Canon CR3 CCDT box is truncated"));
        }
        records.push(CctpRecord {
            image_type: u64::from_be_bytes(
                payload[0..8]
                    .try_into()
                    .map_err(|_| invalid("Canon CR3 CCDT type is truncated"))?,
            ),
            track_index: u32::from_be_bytes(
                payload[12..16]
                    .try_into()
                    .map_err(|_| invalid("Canon CR3 CCDT track index is truncated"))?,
            ),
        });
    }
    Ok(records)
}

#[derive(Clone, Copy, Debug)]
struct Cmp1 {
    version: u16,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
    bit_depth: u8,
    cfa_layout: u8,
    encoding_type: u8,
    levels: u8,
    mdat_header_size: u32,
    median_precision: u8,
}

fn parse_cmp1(cmp1: BoxRef<'_>) -> Result<Cmp1, DecodeError> {
    let payload = cmp1.payload();
    if payload.len() < 32 {
        return Err(invalid("Canon CR3 CMP1 payload is truncated"));
    }
    let bit_depth = payload[24];
    let planes_cfa = payload[25];
    let encoding_levels = payload[26];
    if planes_cfa >> 4 != 4 {
        return Err(DecodeError::Unsupported(
            "Canon CR3 CMP1 plane layout is not four-plane Bayer".into(),
        ));
    }
    let mut median_precision = bit_depth;
    let ext_header = payload.get(32..36).map_or(0, |bytes| {
        u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4]))
    });
    if payload.len() >= 57
        && ext_header & 0x8000_0000 != 0
        && u32::from_be_bytes(payload[52..56].try_into().unwrap_or([0; 4])) & 0x4000_0000 != 0
        && payload.len() > 84
    {
        median_precision = payload[84];
    }
    Ok(Cmp1 {
        version: u16::from_be_bytes(payload[4..6].try_into().unwrap()),
        width: u32::from_be_bytes(payload[8..12].try_into().unwrap()),
        height: u32::from_be_bytes(payload[12..16].try_into().unwrap()),
        tile_width: u32::from_be_bytes(payload[16..20].try_into().unwrap()),
        tile_height: u32::from_be_bytes(payload[20..24].try_into().unwrap()),
        bit_depth,
        cfa_layout: planes_cfa & 0x0f,
        encoding_type: encoding_levels >> 4,
        levels: encoding_levels & 0x0f,
        mdat_header_size: u32::from_be_bytes(payload[28..32].try_into().unwrap()),
        median_precision,
    })
}

fn parse_geometry(
    iad1: Option<BoxRef<'_>>,
    width: u32,
    height: u32,
) -> Result<Geometry, DecodeError> {
    let full = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let Some(iad1) = iad1 else {
        return Ok(Geometry {
            active: full,
            crop: full,
        });
    };
    let payload = iad1.payload();
    if payload.len() < 20 {
        return Err(invalid("Canon CR3 IAD1 payload is truncated"));
    }
    let image_type = u16::from_be_bytes(
        payload[10..12]
            .try_into()
            .map_err(|_| invalid("Canon CR3 IAD1 type is truncated"))?,
    );
    let crop = inclusive_rect(
        read_u16(payload, 16)?,
        read_u16(payload, 18)?,
        read_u16(payload, 20)?,
        read_u16(payload, 22)?,
        width,
        height,
    )?;
    match image_type {
        0 => Ok(Geometry { active: crop, crop }),
        2 => {
            if payload.len() < 48 {
                return Err(invalid("Canon CR3 IAD1 active area is truncated"));
            }
            let active = inclusive_rect(
                read_u16(payload, 40)?,
                read_u16(payload, 42)?,
                read_u16(payload, 44)?,
                read_u16(payload, 46)?,
                width,
                height,
            )?;
            let crop_contained = crop.x >= active.x
                && crop.y >= active.y
                && crop.x.saturating_add(crop.width) <= active.x.saturating_add(active.width)
                && crop.y.saturating_add(crop.height) <= active.y.saturating_add(active.height);
            Ok(Geometry {
                active: if crop_contained { active } else { crop },
                crop,
            })
        }
        other => Err(DecodeError::Unsupported(
            format!("Canon CR3 IAD1 image type {other} is unsupported").into(),
        )),
    }
}

fn inclusive_rect(
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
    width: u32,
    height: u32,
) -> Result<Rect, DecodeError> {
    let right = u32::from(right)
        .checked_add(1)
        .ok_or_else(|| invalid("Canon CR3 IAD1 right endpoint overflow"))?
        .min(width);
    let bottom = u32::from(bottom)
        .checked_add(1)
        .ok_or_else(|| invalid("Canon CR3 IAD1 bottom endpoint overflow"))?
        .min(height);
    let left = u32::from(left);
    let top = u32::from(top);
    if left >= right || top >= bottom || right > width || bottom > height {
        return Err(invalid("Canon CR3 IAD1 rectangle is outside CMP1"));
    }
    Ok(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub struct Cmt {
    pub cmt1: Directory,
    pub cmt2: Directory,
    pub cmt3: Directory,
    pub cmt4: Directory,
}

fn parse_cmt(cr3desc: &BoxRef<'_>) -> Result<Cmt, DecodeError> {
    Ok(Cmt {
        cmt1: required_tiff_child(cr3desc, *b"CMT1")?,
        cmt2: optional_tiff_child(cr3desc, *b"CMT2")?.unwrap_or_default(),
        cmt3: required_tiff_child(cr3desc, *b"CMT3")?,
        cmt4: optional_tiff_child(cr3desc, *b"CMT4")?.unwrap_or_default(),
    })
}

fn required_tiff_child(parent: &BoxRef<'_>, typ: [u8; 4]) -> Result<Directory, DecodeError> {
    optional_tiff_child(parent, typ)?.ok_or_else(|| {
        invalid(format!(
            "Canon CR3 {} box is missing",
            String::from_utf8_lossy(&typ)
        ))
    })
}

pub fn optional_tiff_child(
    parent: &BoxRef<'_>,
    typ: [u8; 4],
) -> Result<Option<Directory>, DecodeError> {
    let Some(child) = child(parent, FourCc::new(typ), 0)? else {
        return Ok(None);
    };
    let payload = if child.payload().starts_with(b"II") || child.payload().starts_with(b"MM") {
        child.payload()
    } else {
        let offset = child
            .payload()
            .windows(2)
            .take(16)
            .position(|pair| pair == b"II" || pair == b"MM")
            .ok_or_else(|| invalid("Canon CR3 CMT payload has no TIFF header"))?;
        &child.payload()[offset..]
    };
    fornax_tiff::parse(payload)
        .map(|parsed| Some(parsed.root().clone()))
        .map_err(|error| invalid(format!("parsing Canon CMT TIFF failed: {error}")))
}

fn parse_color_data(source: &[u8], tracks: &[Track<'_>]) -> Result<ColorDataStatus, DecodeError> {
    let Some(track) = tracks.iter().find(|track| track.kind == TrackKind::Ctmd) else {
        return Ok(ColorDataStatus::Missing);
    };
    let sample = track
        .sample_table
        .sample(0, source.len())
        .map_err(bmff_invalid)?;
    let bytes = source
        .get(sample.offset..sample.offset + sample.size)
        .ok_or_else(|| invalid("Canon CR3 CTMD sample is outside the source"))?;
    let ctmd = color_data::parse_ctmd(bytes)?;
    Ok(ctmd.color_data.unwrap_or(ColorDataStatus::Missing))
}

fn validate_brand(bmff: &Bmff<'_>) -> Result<(), DecodeError> {
    let ftyp = top_child(bmff, FourCc::new(*b"ftyp"))?
        .ok_or_else(|| invalid("Canon CR3 ftyp box is missing"))?;
    if ftyp
        .payload()
        .windows(4)
        .step_by(4)
        .any(|brand| brand == CRX_BRAND.as_bytes())
    {
        Ok(())
    } else {
        Err(DecodeError::Unsupported(
            "Canon CR3 ftyp does not declare the crx brand".into(),
        ))
    }
}

fn child<'a>(
    parent: &BoxRef<'a>,
    typ: FourCc,
    skip: usize,
) -> Result<Option<BoxRef<'a>>, DecodeError> {
    let payload = parent.payload().get(skip..).ok_or_else(|| {
        invalid(format!(
            "{} payload is shorter than {skip} bytes",
            parent.typ()
        ))
    })?;
    let parsed = Bmff::parse(payload).map_err(bmff_invalid)?;
    Ok(parsed
        .boxes()
        .iter()
        .copied()
        .find(|item| item.typ() == typ))
}

fn top_child<'a>(bmff: &Bmff<'a>, typ: FourCc) -> Result<Option<BoxRef<'a>>, DecodeError> {
    bmff.find(&[typ]).map_err(bmff_invalid)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let bytes: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("Canon CR3 IAD1 field is truncated"))?
        .try_into()
        .map_err(|_| invalid("Canon CR3 IAD1 field is truncated"))?;
    Ok(u16::from_be_bytes(bytes))
}

#[cfg(test)]
pub fn parse_geometry_for_test(
    image_type: u16,
    crop_left: u16,
    crop_top: u16,
    crop_right: u16,
    crop_bottom: u16,
    active_left: u16,
    active_top: u16,
    active_right: u16,
    active_bottom: u16,
    width: u32,
    height: u32,
) -> Result<Geometry, DecodeError> {
    let fields_len = if image_type == 2 { 48 } else { 24 };
    let mut payload = vec![0_u8; fields_len];
    payload[10..12].copy_from_slice(&image_type.to_be_bytes());
    payload[16..18].copy_from_slice(&crop_left.to_be_bytes());
    payload[18..20].copy_from_slice(&crop_top.to_be_bytes());
    payload[20..22].copy_from_slice(&crop_right.to_be_bytes());
    payload[22..24].copy_from_slice(&crop_bottom.to_be_bytes());
    if image_type == 2 {
        payload[40..42].copy_from_slice(&active_left.to_be_bytes());
        payload[42..44].copy_from_slice(&active_top.to_be_bytes());
        payload[44..46].copy_from_slice(&active_right.to_be_bytes());
        payload[46..48].copy_from_slice(&active_bottom.to_be_bytes());
    }
    let mut bytes = u32::try_from(payload.len() + 8)
        .map_err(|_| invalid("Canon IAD1 test box exceeds u32"))?
        .to_be_bytes()
        .to_vec();
    bytes.extend_from_slice(b"IAD1");
    bytes.extend_from_slice(&payload);
    let bmff = Bmff::parse(&bytes).map_err(bmff_invalid)?;
    parse_geometry(bmff.boxes().first().copied(), width, height)
}

fn bmff_invalid(error: BmffError) -> DecodeError {
    match error {
        BmffError::InvalidInput(message) => DecodeError::InvalidInput(message),
        BmffError::LimitExceeded(message) => DecodeError::Unsupported(message),
    }
}
