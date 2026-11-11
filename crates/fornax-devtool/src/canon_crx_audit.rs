//! Canon CRX corpus header audit.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use fornax_bmff::{Bmff, BmffLimits, BoxRef, FourCc, SampleTable, SampleTableLimits};

/// CMP1 fields needed to distinguish the accepted lossless shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cmp1Audit {
    /// CMP1 codec version.
    pub version: u16,
    /// Full mosaic width.
    pub width: u32,
    /// Full mosaic height.
    pub height: u32,
    /// Codec tile width.
    pub tile_width: u32,
    /// Codec tile height.
    pub tile_height: u32,
    /// Stored sample precision.
    pub bit_depth: u8,
    /// Codec plane count.
    pub plane_count: u8,
    /// Packed CFA layout identifier.
    pub cfa_layout: u8,
    /// Lossless/lossy encoding discriminator.
    pub encoding_type: u8,
    /// Wavelet decomposition level count.
    pub levels: u8,
    /// MDAT codec-header size.
    pub mdat_header_size: u32,
}

impl Cmp1Audit {
    /// Parses the CMP1 fields needed by the audit.
    ///
    /// # Errors
    ///
    /// Returns a message when the CMP1 payload is truncated.
    pub fn parse(payload: &[u8]) -> Result<Self, String> {
        if payload.len() < 32 {
            return Err("CMP1 payload is shorter than 32 bytes".to_owned());
        }
        let version = u16::from_be_bytes(payload[4..6].try_into().unwrap());
        let width = u32::from_be_bytes(payload[8..12].try_into().unwrap());
        let height = u32::from_be_bytes(payload[12..16].try_into().unwrap());
        let tile_width = u32::from_be_bytes(payload[16..20].try_into().unwrap());
        let tile_height = u32::from_be_bytes(payload[20..24].try_into().unwrap());
        let bit_depth = payload[24];
        let planes_cfa = payload[25];
        let encoding_levels = payload[26];
        let mdat_header_size = u32::from_be_bytes(payload[28..32].try_into().unwrap());
        Ok(Self {
            version,
            width,
            height,
            tile_width,
            tile_height,
            bit_depth,
            plane_count: planes_cfa >> 4,
            cfa_layout: planes_cfa & 0x0f,
            encoding_type: encoding_levels >> 4,
            levels: encoding_levels & 0x0f,
            mdat_header_size,
        })
    }

    /// Returns whether this CMP1 shape is in the validated lossless scope.
    ///
    /// The corpus audit accepts only the lossless tuple observed for Canon
    /// RAW files. C-RAW uses version `0x0200` and three wavelet levels.
    pub const fn is_lossless(self) -> bool {
        self.version == 0x0100
            && self.bit_depth >= 8
            && self.bit_depth <= 16
            && self.plane_count == 4
            && self.cfa_layout <= 3
            && self.encoding_type == 0
            && self.levels == 0
            && self.mdat_header_size == 0x70
            && self.width > 0
            && self.height > 0
            && self.tile_width > 0
            && self.tile_height > 0
    }
}

/// One audited Canon file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRow {
    /// Corpus-relative or supplied path.
    pub path: String,
    /// Number of top-level movie tracks.
    pub track_count: usize,
    /// Selected CMP1 values.
    pub cmp1: Cmp1Audit,
    /// COLORDATA version when it could be located.
    pub color_data_version: Option<i16>,
    /// Lossless-scope classification.
    pub lossless: bool,
}

/// Audits one CR3/CRM file from disk.
///
/// # Errors
///
/// Returns an I/O or parse error when the file cannot be audited.
pub fn audit_file(path: &Path) -> Result<AuditRow, String> {
    let bytes = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    audit_bytes(&bytes, &path.display().to_string())
}

/// Audits one CR3/CRM byte stream.
///
/// # Errors
///
/// Returns a message when the box tree has no usable CRAW/CMP1 track.
pub fn audit_bytes(source: &[u8], path: &str) -> Result<AuditRow, String> {
    let bmff = Bmff::parse(source).map_err(|error| error.to_string())?;
    let track_count = top_level_track_count(&bmff)?;
    let color_data_version = audit_color_data_version(source, &bmff).ok().flatten();
    let mut cmp1 = None;
    walk_boxes(&bmff, 0, &mut |item| {
        if item.typ() == FourCc::new(*b"CMP1") {
            let Ok(candidate) = Cmp1Audit::parse(item.payload()) else {
                return;
            };
            let area = u64::from(candidate.width) * u64::from(candidate.height);
            let replace = cmp1.is_none_or(|current: Cmp1Audit| {
                area > u64::from(current.width) * u64::from(current.height)
            });
            if replace {
                cmp1 = Some(candidate);
            }
        }
    })?;
    let cmp1 = cmp1.ok_or_else(|| "no usable CMP1 box found".to_owned())?;
    Ok(AuditRow {
        path: path.to_owned(),
        track_count,
        cmp1,
        color_data_version,
        lossless: cmp1.is_lossless(),
    })
}

/// Audits files from a pointer manifest.
///
/// # Errors
///
/// Returns an error when the manifest or any selected CR3/CRM file cannot be
/// read or parsed.
pub fn audit_manifest(corpus: &Path, manifest: &Path) -> Result<Vec<AuditRow>, String> {
    let mut reader = csv::Reader::from_path(manifest)
        .map_err(|error| format!("opening {}: {error}", manifest.display()))?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|error| format!("reading {}: {error}", manifest.display()))?;
        let path = record
            .get(0)
            .ok_or_else(|| "manifest row has no path".to_owned())?;
        let extension = Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("cr3") || extension.eq_ignore_ascii_case("crm") {
            rows.push(audit_file(&corpus.join(path))?);
        }
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(rows)
}

/// Renders audit rows as CSV.
pub fn render_csv(rows: &[AuditRow]) -> String {
    let mut output = String::from(
        "path,track_count,cmp1_version,encoding_type,levels,bit_depth,tile_width,tile_height,cfas,color_data_version,classification\n",
    );
    for row in rows {
        let _ = writeln!(
            output,
            "{},{},{:#06x},{},{},{},{},{},{:#x},{},{}",
            csv_quote(&row.path),
            row.track_count,
            row.cmp1.version,
            row.cmp1.encoding_type,
            row.cmp1.levels,
            row.cmp1.bit_depth,
            row.cmp1.tile_width,
            row.cmp1.tile_height,
            row.cmp1.cfa_layout,
            row.color_data_version
                .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
            if row.lossless {
                "lossless"
            } else {
                "unsupported-lossy-or-ambiguous"
            }
        );
    }
    output
}

/// Summarizes the number of lossless and lossy/ambiguous rows.
pub fn summarize(rows: &[AuditRow]) -> (usize, usize) {
    rows.iter().fold((0, 0), |(lossless, other), row| {
        if row.lossless {
            (lossless + 1, other)
        } else {
            (lossless, other + 1)
        }
    })
}

fn top_level_track_count(bmff: &Bmff<'_>) -> Result<usize, String> {
    let Some(moov) = bmff
        .find(&[FourCc::new(*b"moov")])
        .map_err(|error| error.to_string())?
    else {
        return Err("no moov box found".to_owned());
    };
    Ok(moov
        .children(BmffLimits::default())
        .map_err(|error| error.to_string())?
        .iter()
        .filter(|item| item.typ() == FourCc::new(*b"trak"))
        .count())
}

fn audit_color_data_version(source: &[u8], bmff: &Bmff<'_>) -> Result<Option<i16>, String> {
    let Some(moov) = bmff
        .find(&[FourCc::new(*b"moov")])
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let tracks = moov
        .children(BmffLimits::default())
        .map_err(|error| error.to_string())?;
    for trak in tracks
        .iter()
        .filter(|item| item.typ() == FourCc::new(*b"trak"))
    {
        let Some(mdia) = child(*trak, FourCc::new(*b"mdia"), 0)? else {
            continue;
        };
        let Some(minf) = child(mdia, FourCc::new(*b"minf"), 0)? else {
            continue;
        };
        let Some(stbl) = child(minf, FourCc::new(*b"stbl"), 0)? else {
            continue;
        };
        let Some(stsd) = child(stbl, FourCc::new(*b"stsd"), 0)? else {
            continue;
        };
        if child(stsd, FourCc::new(*b"CTMD"), 8)?.is_none() {
            continue;
        }
        let stsc = child(stbl, FourCc::new(*b"stsc"), 0)?
            .ok_or_else(|| "CTMD stsc is missing".to_owned())?;
        let stsz = child(stbl, FourCc::new(*b"stsz"), 0)?
            .ok_or_else(|| "CTMD stsz is missing".to_owned())?;
        let (chunk_type, chunk_offsets) =
            if let Some(value) = child(stbl, FourCc::new(*b"stco"), 0)? {
                (*b"stco", value)
            } else {
                (
                    *b"co64",
                    child(stbl, FourCc::new(*b"co64"), 0)?
                        .ok_or_else(|| "CTMD chunk offsets are missing".to_owned())?,
                )
            };
        let table = SampleTable::parse(
            stsc.payload(),
            stsz.payload(),
            chunk_type,
            chunk_offsets.payload(),
            SampleTableLimits::default(),
        )
        .map_err(|error| error.to_string())?;
        let sample = table
            .sample(0, source.len())
            .map_err(|error| error.to_string())?;
        return color_data_version_from_ctmd(
            source
                .get(sample.offset..sample.offset + sample.size)
                .ok_or_else(|| "CTMD sample exceeds source".to_owned())?,
        );
    }
    Ok(None)
}

fn color_data_version_from_ctmd(data: &[u8]) -> Result<Option<i16>, String> {
    let mut position = 0_usize;
    while position + 12 <= data.len() {
        let size = usize::try_from(read_u32_le(data, position)?)
            .map_err(|_| "CTMD record size exceeds usize".to_owned())?;
        if size < 12 || position + size > data.len() {
            break;
        }
        if read_u16_le(data, position + 4)? == 8 {
            return color_data_version_from_record(&data[position + 12..position + size]);
        }
        position += size;
    }
    Ok(None)
}

fn color_data_version_from_record(data: &[u8]) -> Result<Option<i16>, String> {
    let mut position = 0_usize;
    while position + 8 <= data.len() {
        let size = usize::try_from(read_u32_le(data, position)?)
            .map_err(|_| "CTMD block size exceeds usize".to_owned())?;
        if size < 8 || position + size > data.len() {
            return Err("CTMD block is malformed".to_owned());
        }
        if read_u16_le(data, position + 4)? == 0x927c {
            let block = &data[position + 8..position + size];
            let order = match block.get(..2) {
                Some(b"II") => fornax_tiff::ByteOrder::Little,
                Some(b"MM") => fornax_tiff::ByteOrder::Big,
                _ => return Err("CTMD TIFF header is missing".to_owned()),
            };
            let ifd_offset = match order {
                fornax_tiff::ByteOrder::Little => {
                    u32::from_le_bytes(block[4..8].try_into().map_err(|_| "TIFF IFD offset")?)
                }
                fornax_tiff::ByteOrder::Big => {
                    u32::from_be_bytes(block[4..8].try_into().map_err(|_| "TIFF IFD offset")?)
                }
            };
            let directory = fornax_tiff::parse_ifd_at(block, u64::from(ifd_offset), order, 0)
                .map_err(|error| error.to_string())?;
            let Some(value) = directory.get(&fornax_tiff::Tag(0x4001)) else {
                return Ok(None);
            };
            let values = value
                .clone()
                .into_u64_vec()
                .map_err(|_| "ColorData is not an integer list".to_owned())?;
            let version = values
                .first()
                .copied()
                .ok_or_else(|| "ColorData is empty".to_owned())?;
            return u16::try_from(version)
                .map(|value| Some(value as i16))
                .map_err(|_| "ColorData version exceeds u16".to_owned());
        }
        position += size;
    }
    Ok(None)
}

fn child(parent: BoxRef<'_>, typ: FourCc, skip: usize) -> Result<Option<BoxRef<'_>>, String> {
    let payload = parent
        .payload()
        .get(skip..)
        .ok_or_else(|| format!("{} payload is shorter than {skip}", parent.typ()))?;
    let bmff = Bmff::parse(payload).map_err(|error| error.to_string())?;
    Ok(bmff.boxes().iter().copied().find(|item| item.typ() == typ))
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or_else(|| "CTMD u16 is truncated".to_owned())?
        .try_into()
        .map_err(|_| "CTMD u16 is truncated".to_owned())?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| "CTMD u32 is truncated".to_owned())?
        .try_into()
        .map_err(|_| "CTMD u32 is truncated".to_owned())?;
    Ok(u32::from_le_bytes(bytes))
}

fn walk_boxes(
    bmff: &Bmff<'_>,
    depth: u8,
    visitor: &mut impl FnMut(BoxRef<'_>),
) -> Result<(), String> {
    const CR3_DESCRIPTION_UUID: [u8; 16] = [
        0x85, 0xc0, 0xb6, 0x87, 0x82, 0x0f, 0x11, 0xe0, 0x81, 0x11, 0xf4, 0xce, 0x46, 0x2b, 0x6a,
        0x48,
    ];
    if depth > 16 {
        return Err("box nesting exceeds audit limit".to_owned());
    }
    for item in bmff.boxes() {
        visitor(*item);
        let typ = item.typ().as_bytes();
        let skip = match &typ {
            b"stsd" => Some(8),
            b"CRAW" => Some(82),
            b"CDI1" => Some(4),
            b"CCTP" => Some(12),
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => Some(0),
            b"uuid" if item.uuid() == Some(CR3_DESCRIPTION_UUID) => Some(0),
            _ => None,
        };
        let Some(skip) = skip else {
            continue;
        };
        let payload = item
            .payload()
            .get(skip..)
            .ok_or_else(|| format!("{} payload is shorter than {skip} bytes", item.typ()))?;
        let child = Bmff::parse(payload)
            .map_err(|error| format!("parsing {} children: {error}", item.typ()))?;
        walk_boxes(&child, depth + 1, visitor)?;
    }
    Ok(())
}

fn csv_quote(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp1(version: u16, encoding_type: u8, levels: u8) -> Cmp1Audit {
        Cmp1Audit {
            version,
            width: 100,
            height: 80,
            tile_width: 100,
            tile_height: 80,
            bit_depth: 14,
            plane_count: 4,
            cfa_layout: 0,
            encoding_type,
            levels,
            mdat_header_size: 0x70,
        }
    }

    #[test]
    fn classifies_only_the_observed_lossless_cmp1_shape() {
        assert!(cmp1(0x0100, 0, 0).is_lossless());
        assert!(!cmp1(0x0200, 0, 3).is_lossless());
        assert!(!cmp1(0x0200, 0, 0).is_lossless());
        assert!(!cmp1(0x0100, 1, 0).is_lossless());
        assert!(!cmp1(0x0100, 0, 1).is_lossless());
    }

    #[test]
    fn renders_classification_counts() {
        let mut first = cmp1(0x0100, 0, 0);
        let second = cmp1(0x0200, 0, 3);
        let rows = [
            AuditRow {
                path: "lossless.cr3".into(),
                track_count: 2,
                cmp1: first,
                color_data_version: Some(33),
                lossless: first.is_lossless(),
            },
            AuditRow {
                path: "lossy,one.cr3".into(),
                track_count: 2,
                cmp1: second,
                color_data_version: Some(48),
                lossless: second.is_lossless(),
            },
        ];
        assert_eq!(summarize(&rows), (1, 1));
        let csv = render_csv(&rows);
        assert!(csv.contains("\"lossy,one.cr3\""));
        assert!(csv.contains(",unsupported-lossy-or-ambiguous"));

        first.cfa_layout = 4;
        assert!(!first.is_lossless());
    }
}
