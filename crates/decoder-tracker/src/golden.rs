use std::collections::BTreeMap;

use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::metadata::{ColorTransform, Matrix3x3, ProcessMetadata};
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_raw_decoder::Decoder;
use fornax_tiff::{Directory, Value};
use image::math::Rect;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::csvio::write_manifest_csv;
use crate::manifest::derive_identity;
use crate::model::{BaselineRow, ManifestRow, Result, TrackerError};

pub const GOLDEN_HEADER: [&str; 10] = [
    "path",
    "sample_type",
    "width",
    "height",
    "raw_sha256",
    "cropped_width",
    "cropped_height",
    "cropped_sha256",
    "process_sha256",
    "dng_sha256",
];
pub const GOLDEN_COVERAGE_HEADER: [&str; 19] = [
    "format",
    "compression_name",
    "compression_lossless",
    "compression_id",
    "bits_per_sample",
    "sample_format",
    "storage",
    "predictor",
    "cfa",
    "coverage",
    "passing_count",
    "golden_count",
    "golden_paths",
    "representative_path",
    "representative_camera",
    "representative_size",
    "representative_sha256",
    "representative_width",
    "representative_height",
];

const RAW_DOMAIN: &[u8] = b"fornax.raw_mosaic.v1";
const CROPPED_DOMAIN: &[u8] = b"fornax.cropped_mosaic.f32.v1";
const PROCESS_DOMAIN: &[u8] = b"fornax.process_metadata.v1";
const DNG_DOMAIN: &[u8] = b"fornax.dng_directory.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GoldenRow {
    pub path: String,
    pub sample_type: String,
    pub width: u32,
    pub height: u32,
    pub raw_sha256: String,
    pub cropped_width: u32,
    pub cropped_height: u32,
    pub cropped_sha256: String,
    pub process_sha256: String,
    pub dng_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenCsv {
    pub rows: Vec<GoldenRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GoldenCoverageRow {
    pub format: String,
    pub compression_name: String,
    pub compression_lossless: u8,
    pub compression_id: Option<u32>,
    pub bits_per_sample: Option<u16>,
    pub sample_format: String,
    pub storage: String,
    pub predictor: Option<u16>,
    pub cfa: String,
    pub coverage: String,
    pub passing_count: usize,
    pub golden_count: usize,
    pub golden_paths: String,
    pub representative_path: String,
    pub representative_camera: String,
    pub representative_size: u64,
    pub representative_sha256: String,
    pub representative_width: u32,
    pub representative_height: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GoldenCoverageSummary {
    pub covered: usize,
    pub missing: usize,
    pub redundant: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenCoverage {
    pub rows: Vec<GoldenCoverageRow>,
    pub summary: GoldenCoverageSummary,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CoverageKey {
    format: String,
    compression_name: String,
    bits_per_sample: Option<u16>,
    sample_format: String,
    storage: String,
    predictor: Option<u16>,
}

pub fn build_golden_coverage(
    baseline: &[BaselineRow],
    corpus_manifest: &[ManifestRow],
    golden_manifest: &[ManifestRow],
) -> Result<GoldenCoverage> {
    build_golden_coverage_with_exclusions(baseline, corpus_manifest, golden_manifest, &[])
}

pub fn build_golden_coverage_with_exclusions(
    baseline: &[BaselineRow],
    corpus_manifest: &[ManifestRow],
    golden_manifest: &[ManifestRow],
    excluded_paths: &[String],
) -> Result<GoldenCoverage> {
    let mut manifest = BTreeMap::new();
    for row in corpus_manifest {
        if manifest.insert(row.path.as_str(), row).is_some() {
            return Err(TrackerError::new(format!(
                "corpus manifest contains duplicate path {}",
                row.path
            )));
        }
    }
    let excluded = excluded_paths
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for path in &excluded {
        if !manifest.contains_key(path) {
            return Err(TrackerError::new(format!(
                "excluded path {path} is absent from the corpus manifest"
            )));
        }
    }

    let mut baseline_index = BTreeMap::new();
    for row in baseline {
        if baseline_index.insert(row.path.as_str(), row).is_some() {
            return Err(TrackerError::new(format!(
                "baseline contains duplicate path {}",
                row.path
            )));
        }
        if !manifest.contains_key(row.path.as_str()) {
            return Err(TrackerError::new(format!(
                "baseline path {} is absent from the corpus manifest",
                row.path
            )));
        }
    }

    let mut golden_paths = BTreeMap::new();
    for row in golden_manifest {
        if golden_paths.insert(row.path.as_str(), row).is_some() {
            return Err(TrackerError::new(format!(
                "golden manifest contains duplicate path {}",
                row.path
            )));
        }
        let Some(baseline_row) = baseline_index.get(row.path.as_str()) else {
            return Err(TrackerError::new(format!(
                "golden path {} is absent from the baseline",
                row.path
            )));
        };
        if !fully_passing(baseline_row) {
            return Err(TrackerError::new(format!(
                "golden path {} is not passing",
                row.path
            )));
        }
        let corpus_row = manifest[row.path.as_str()];
        if row.size != corpus_row.size
            || row.sha256 != corpus_row.sha256
            || row.format != corpus_row.format
        {
            return Err(TrackerError::new(format!(
                "golden path {} does not match the corpus manifest",
                row.path
            )));
        }
    }

    let mut groups = BTreeMap::<CoverageKey, Vec<&BaselineRow>>::new();
    for row in baseline.iter().filter(|row| {
        fully_passing(row)
            && row.compression_lossless == Some(1)
            && !excluded.contains(row.path.as_str())
    }) {
        groups
            .entry(CoverageKey {
                format: row.format.clone(),
                compression_name: row.compression_name.clone(),
                bits_per_sample: row.bits_per_sample,
                sample_format: row.sample_format.clone(),
                storage: row.storage.clone(),
                predictor: row.predictor,
            })
            .or_default()
            .push(row);
    }

    let mut summary = GoldenCoverageSummary::default();
    let rows = groups
        .into_iter()
        .map(|(key, mut passing)| {
            passing.sort_by(|left, right| {
                let left_pixels = u64::from(left.width) * u64::from(left.height);
                let right_pixels = u64::from(right.width) * u64::from(right.height);
                left_pixels
                    .cmp(&right_pixels)
                    .then_with(|| {
                        manifest[left.path.as_str()]
                            .size
                            .cmp(&manifest[right.path.as_str()].size)
                    })
                    .then_with(|| left.path.cmp(&right.path))
            });
            let selected = passing
                .iter()
                .filter_map(|row| golden_paths.get(row.path.as_str()).copied())
                .collect::<Vec<_>>();
            let coverage = match selected.len() {
                0 => {
                    summary.missing += 1;
                    "missing"
                }
                1 => {
                    summary.covered += 1;
                    "covered"
                }
                _ => {
                    summary.redundant += 1;
                    "redundant"
                }
            };
            let representative = passing[0];
            let representative_manifest = manifest[representative.path.as_str()];
            GoldenCoverageRow {
                format: key.format,
                compression_name: key.compression_name,
                compression_lossless: 1,
                compression_id: representative.compression_id,
                bits_per_sample: key.bits_per_sample,
                sample_format: key.sample_format,
                storage: key.storage,
                predictor: key.predictor,
                cfa: representative.cfa.clone(),
                coverage: coverage.to_owned(),
                passing_count: passing.len(),
                golden_count: selected.len(),
                golden_paths: selected
                    .iter()
                    .map(|row| row.path.as_str())
                    .collect::<Vec<_>>()
                    .join(";"),
                representative_path: representative.path.clone(),
                representative_camera: representative.camera.clone(),
                representative_size: representative_manifest.size,
                representative_sha256: representative_manifest.sha256.clone(),
                representative_width: representative.width,
                representative_height: representative.height,
            }
        })
        .collect();
    Ok(GoldenCoverage { rows, summary })
}

pub fn write_golden_coverage_csv(rows: &[GoldenCoverageRow]) -> Result<Vec<u8>> {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| {
        (
            &left.format,
            &left.compression_name,
            left.compression_lossless,
            left.bits_per_sample,
            &left.sample_format,
            &left.storage,
            left.predictor,
        )
            .cmp(&(
                &right.format,
                &right.compression_name,
                right.compression_lossless,
                right.bits_per_sample,
                &right.sample_format,
                &right.storage,
                right.predictor,
            ))
    });
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .terminator(Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(GOLDEN_COVERAGE_HEADER)?;
    for row in rows {
        writer.serialize(row)?;
    }
    Ok(writer
        .into_inner()
        .map_err(csv::IntoInnerError::into_error)?)
}

pub fn write_golden_selection_csv(rows: &[GoldenCoverageRow]) -> Result<Vec<u8>> {
    let rows = rows
        .iter()
        .map(|row| {
            let (vendor, camera, model, filename) = derive_identity(&row.representative_path)?;
            let format = row
                .representative_path
                .rsplit_once('.')
                .map_or_else(String::new, |(_, extension)| extension.to_ascii_uppercase());
            Ok(ManifestRow {
                sha256: row.representative_sha256.clone(),
                size: row.representative_size,
                path: row.representative_path.clone(),
                vendor,
                camera,
                model,
                format,
                filename,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    write_manifest_csv(&rows)
}

const fn fully_passing(row: &BaselineRow) -> bool {
    row.decode_pass == 1
        && row.raw_mosaic_pass == 1
        && row.process_metadata_pass == 1
        && row.handoff_pass == 1
}

pub fn decode_golden(path: &str, input: &std::path::Path) -> Result<GoldenRow> {
    let decoder = Decoder::decode(input).map_err(|error| {
        TrackerError::new(format!("decoding {} failed: {error}", input.display()))
    })?;
    let raw = decoder.raw_mosaic().map_err(|error| {
        TrackerError::new(format!("raw mosaic {} failed: {error}", input.display()))
    })?;
    let cropped = decoder.cropped_mosaic::<f32>().map_err(|error| {
        TrackerError::new(format!(
            "cropped mosaic {} failed: {error}",
            input.display()
        ))
    })?;
    let process = decoder.process_metadata().map_err(|error| {
        TrackerError::new(format!(
            "process metadata {} failed: {error}",
            input.display()
        ))
    })?;
    let dng = decoder.dng_metadata().map_err(|error| {
        TrackerError::new(format!("DNG metadata {} failed: {error}", input.display()))
    })?;
    Ok(GoldenRow::from_decoded(
        path,
        &raw,
        cropped.image(),
        &process,
        &dng,
    ))
}

impl GoldenRow {
    pub fn from_decoded(
        path: &str,
        raw: &RawMosaic,
        cropped: &MonoImage<f32>,
        process: &ProcessMetadata,
        dng: &Directory,
    ) -> Self {
        let (width, height) = raw.dimensions();
        let (cropped_width, cropped_height) = cropped.dimensions();
        Self {
            path: path.to_owned(),
            sample_type: raw_sample_type(raw).to_owned(),
            width,
            height,
            raw_sha256: raw_sha256(raw),
            cropped_width,
            cropped_height,
            cropped_sha256: cropped_sha256(cropped),
            process_sha256: process_sha256(process),
            dng_sha256: dng_sha256(dng),
        }
    }

    fn validate(&self) -> Result<()> {
        if self.path.is_empty() {
            return Err(TrackerError::new("golden row has an empty path"));
        }
        for (field, value) in [
            ("raw_sha256", self.raw_sha256.as_str()),
            ("cropped_sha256", self.cropped_sha256.as_str()),
            ("process_sha256", self.process_sha256.as_str()),
            ("dng_sha256", self.dng_sha256.as_str()),
        ] {
            if value.len() != 64 || hex::decode(value).is_err() {
                return Err(TrackerError::new(format!(
                    "{} has invalid {field}",
                    self.path
                )));
            }
        }
        Ok(())
    }
}

pub fn write_golden_csv(rows: &[GoldenRow]) -> Result<Vec<u8>> {
    let rows = index_rows(rows)?.into_values().collect::<Vec<_>>();
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .terminator(Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(GOLDEN_HEADER)?;
    for row in rows {
        writer.serialize(row)?;
    }
    Ok(writer
        .into_inner()
        .map_err(csv::IntoInnerError::into_error)?)
}

pub fn read_golden_csv<A>(input: A) -> Result<GoldenCsv>
where
    A: AsRef<[u8]>,
{
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(input.as_ref());
    validate_header(reader.headers()?)?;
    let rows = reader
        .deserialize::<GoldenRow>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let rows = index_rows(&rows)?.into_values().collect();
    Ok(GoldenCsv { rows })
}

pub fn check_golden_csv(actual: &[GoldenRow], expected: &GoldenCsv) -> Result<()> {
    let actual = index_rows(actual)?;
    let expected = index_rows(&expected.rows)?;
    let missing = expected
        .keys()
        .filter(|path| !actual.contains_key(*path))
        .cloned()
        .collect::<Vec<_>>();
    let extra = actual
        .keys()
        .filter(|path| !expected.contains_key(*path))
        .cloned()
        .collect::<Vec<_>>();
    let changed = expected
        .iter()
        .filter(|(path, expected)| actual.get(*path).is_some_and(|actual| actual != *expected))
        .map(|(path, _)| path.clone())
        .collect::<Vec<_>>();
    if missing.is_empty() && extra.is_empty() && changed.is_empty() {
        return Ok(());
    }
    Err(TrackerError::new(format!(
        "golden state differs: {} missing, {} extra, {} changed; \
         missing={}; extra={}; changed={}",
        missing.len(),
        extra.len(),
        changed.len(),
        missing.join(","),
        extra.join(","),
        changed.join(",")
    )))
}

fn index_rows(rows: &[GoldenRow]) -> Result<BTreeMap<String, GoldenRow>> {
    let mut index = BTreeMap::new();
    for row in rows {
        row.validate()?;
        if index.insert(row.path.clone(), row.clone()).is_some() {
            return Err(TrackerError::new(format!(
                "golden CSV contains duplicate path {}",
                row.path
            )));
        }
    }
    Ok(index)
}

fn validate_header(actual: &StringRecord) -> Result<()> {
    if actual.len() != GOLDEN_HEADER.len()
        || !actual
            .iter()
            .zip(GOLDEN_HEADER)
            .all(|(actual, expected)| actual == expected)
    {
        return Err(TrackerError::new(
            "golden CSV header does not match the canonical schema",
        ));
    }
    Ok(())
}

const fn raw_sample_type(raw: &RawMosaic) -> &'static str {
    match raw {
        RawMosaic::U8(_) => "u8",
        RawMosaic::U16(_) => "u16",
        RawMosaic::U32(_) => "u32",
        RawMosaic::F32(_) => "f32",
    }
}

pub(crate) fn raw_sha256(raw: &RawMosaic) -> String {
    match raw {
        RawMosaic::U8(image) => hash_samples(
            RAW_DOMAIN,
            "u8",
            image.dimensions(),
            image.as_raw().iter().flat_map(|value| value.to_le_bytes()),
        ),
        RawMosaic::U16(image) => hash_samples(
            RAW_DOMAIN,
            "u16",
            image.dimensions(),
            image.as_raw().iter().flat_map(|value| value.to_le_bytes()),
        ),
        RawMosaic::U32(image) => hash_samples(
            RAW_DOMAIN,
            "u32",
            image.dimensions(),
            image.as_raw().iter().flat_map(|value| value.to_le_bytes()),
        ),
        RawMosaic::F32(image) => hash_samples(
            RAW_DOMAIN,
            "f32",
            image.dimensions(),
            image
                .as_raw()
                .iter()
                .flat_map(|value| value.to_bits().to_le_bytes()),
        ),
    }
}

fn cropped_sha256(image: &MonoImage<f32>) -> String {
    hash_samples(
        CROPPED_DOMAIN,
        "f32",
        image.dimensions(),
        image
            .as_raw()
            .iter()
            .flat_map(|value| value.to_bits().to_le_bytes()),
    )
}

fn hash_samples<I>(
    domain: &[u8],
    sample_type: &str,
    (width, height): (u32, u32),
    samples: I,
) -> String
where
    I: IntoIterator<Item = u8>,
{
    let mut hash = Sha256::new();
    update_bytes(&mut hash, domain);
    update_bytes(&mut hash, sample_type.as_bytes());
    hash.update(width.to_le_bytes());
    hash.update(height.to_le_bytes());
    for byte in samples {
        hash.update([byte]);
    }
    hex::encode(hash.finalize())
}

fn process_sha256(metadata: &ProcessMetadata) -> String {
    let mut hash = Sha256::new();
    update_bytes(&mut hash, PROCESS_DOMAIN);
    update_rect(&mut hash, metadata.active_area);
    update_rect(&mut hash, metadata.default_crop);
    hash.update(metadata.levels.black.dimensions().0.to_le_bytes());
    hash.update(metadata.levels.black.dimensions().1.to_le_bytes());
    for value in metadata.levels.black.as_raw() {
        hash.update(value.to_bits().to_le_bytes());
    }
    update_f64_vec(&mut hash, &metadata.levels.black_delta_h);
    update_f64_vec(&mut hash, &metadata.levels.black_delta_v);
    hash.update(metadata.levels.white.to_bits().to_le_bytes());
    update_bytes(&mut hash, metadata.bayer_pattern.to_string().as_bytes());
    update_color_transform(&mut hash, &metadata.color_calibration.resolve());
    hex::encode(hash.finalize())
}

fn dng_sha256(metadata: &Directory) -> String {
    let mut hash = Sha256::new();
    update_bytes(&mut hash, DNG_DOMAIN);
    hash.update((metadata.len() as u64).to_le_bytes());
    for (tag, value) in metadata {
        hash.update(tag.0.to_le_bytes());
        update_value(&mut hash, value);
    }
    hex::encode(hash.finalize())
}

fn update_color_transform(hash: &mut Sha256, transform: &ColorTransform) {
    for value in transform.neutral {
        hash.update(value.to_bits().to_le_bytes());
    }
    for value in transform.gains {
        hash.update(value.to_bits().to_le_bytes());
    }
    hash.update(transform.cct.to_bits().to_le_bytes());
    update_matrix(hash, &transform.cam_to_xyz_d50);
    update_matrix(hash, &transform.cam_to_xyz_d50_post_wb);
}

fn update_matrix(hash: &mut Sha256, matrix: &Matrix3x3) {
    for row in matrix {
        for value in row {
            hash.update(value.to_bits().to_le_bytes());
        }
    }
}

fn update_rect(hash: &mut Sha256, rect: Rect) {
    hash.update(rect.x.to_le_bytes());
    hash.update(rect.y.to_le_bytes());
    hash.update(rect.width.to_le_bytes());
    hash.update(rect.height.to_le_bytes());
}

fn update_f64_vec(hash: &mut Sha256, values: &[f64]) {
    hash.update((values.len() as u64).to_le_bytes());
    for value in values {
        hash.update(value.to_bits().to_le_bytes());
    }
}

fn update_value(hash: &mut Sha256, value: &Value) {
    match value {
        Value::Byte(value) => update_tagged(hash, b"byte", &[*value]),
        Value::SignedByte(value) => update_tagged(hash, b"signed-byte", &value.to_le_bytes()),
        Value::Short(value) => update_tagged(hash, b"short", &value.to_le_bytes()),
        Value::SignedShort(value) => update_tagged(hash, b"signed-short", &value.to_le_bytes()),
        Value::Unsigned(value) => update_tagged(hash, b"unsigned", &value.to_le_bytes()),
        Value::Signed(value) => update_tagged(hash, b"signed", &value.to_le_bytes()),
        Value::UnsignedBig(value) => update_tagged(hash, b"unsigned-big", &value.to_le_bytes()),
        Value::SignedBig(value) => update_tagged(hash, b"signed-big", &value.to_le_bytes()),
        Value::Float(value) => update_tagged(hash, b"float", &value.to_bits().to_le_bytes()),
        Value::Double(value) => update_tagged(hash, b"double", &value.to_bits().to_le_bytes()),
        Value::Rational(numerator, denominator) => {
            update_tagged_pair(hash, b"rational", *numerator, *denominator);
        }
        Value::SRational(numerator, denominator) => {
            update_signed_tagged_pair(hash, b"srational", *numerator, *denominator);
        }
        Value::RationalBig(numerator, denominator) => {
            update_signed_big_tagged_pair(hash, b"rational-big", *numerator, *denominator);
        }
        Value::SRationalBig(numerator, denominator) => {
            update_signed_big_tagged_pair(hash, b"srational-big", *numerator, *denominator);
        }
        Value::Ifd(value) => update_tagged(hash, b"ifd", &value.to_le_bytes()),
        Value::IfdBig(value) => update_tagged(hash, b"ifd-big", &value.to_le_bytes()),
        Value::Ascii(value) => update_tagged(hash, b"ascii", value.as_bytes()),
        Value::Bytes(values) => {
            update_bytes(hash, b"list");
            hash.update((values.len() as u64).to_le_bytes());
            for value in values {
                update_value(hash, &Value::Byte(*value));
            }
        }
        Value::List(values) => {
            update_bytes(hash, b"list");
            hash.update((values.len() as u64).to_le_bytes());
            for value in values {
                update_value(hash, value);
            }
        }
    }
}

fn update_tagged(hash: &mut Sha256, tag: &[u8], bytes: &[u8]) {
    update_bytes(hash, tag);
    update_bytes(hash, bytes);
}

fn update_tagged_pair(hash: &mut Sha256, tag: &[u8], numerator: u32, denominator: u32) {
    let mut bytes = [0_u8; 8];
    bytes[..4].copy_from_slice(&numerator.to_le_bytes());
    bytes[4..].copy_from_slice(&denominator.to_le_bytes());
    update_tagged(hash, tag, &bytes);
}

fn update_signed_tagged_pair(hash: &mut Sha256, tag: &[u8], numerator: i32, denominator: i32) {
    let mut bytes = [0_u8; 8];
    bytes[..4].copy_from_slice(&numerator.to_le_bytes());
    bytes[4..].copy_from_slice(&denominator.to_le_bytes());
    update_tagged(hash, tag, &bytes);
}

fn update_signed_big_tagged_pair(hash: &mut Sha256, tag: &[u8], numerator: i64, denominator: i64) {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&numerator.to_le_bytes());
    bytes[8..].copy_from_slice(&denominator.to_le_bytes());
    update_tagged(hash, tag, &bytes);
}

fn update_bytes(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}
