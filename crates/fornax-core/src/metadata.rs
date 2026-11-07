use image::math::Rect;
use num_enum::{FromPrimitive, IntoPrimitive};

use crate::primitive::MonoImage;
use crate::protocol::IPostProcessMetadata;
use crate::{BayerPattern, Primitive};

/// A row-major 3x3 color matrix.
pub type Matrix3x3 = [[f64; 3]; 3];
/// A 3-channel vector (R, G, B or X, Y, Z).
pub type Vec3 = [f64; 3];

// ---------------------------------------------------------------------------
// ProcessMetadata
// ---------------------------------------------------------------------------

/// Normalized metadata for a single-sample Bayer RAW image.
///
/// The geometry and level fields correspond to the same concepts as the
/// corresponding DNG raw-image tags. Coordinates use pixels with `x` as the
/// column and `y` as the row.
///
/// Color data is *not* a copy of the DNG tags: decoders fill a
/// [`RawColorMeta`] with whatever the file provides and
/// [`ColorCalibration::new`] normalizes it.
#[derive(Clone, Debug)]
pub struct ProcessMetadata<T>
where
    T: Primitive,
{
    /// Usable sensor area, corresponding to DNG `ActiveArea`.
    ///
    /// The origin is relative to the full raw frame. In DNG tag order this is
    /// `(top, left, bottom, right)`; this rectangle exposes the same area as
    /// `(x, y, width, height)`.
    pub active_area: Rect,

    /// Recommended output crop from DNG `DefaultCropOrigin` and
    /// `DefaultCropSize`.
    ///
    /// The origin and size are relative to [`ProcessMetadata::active_area`].
    pub default_crop: Rect,

    pub levels: Levels<T>,

    /// CFA layout described by DNG `CFARepeatPatternDim`, `CFAPattern`, and
    /// `CFAPlaneColor`. Cropped.
    pub bayer_pattern: BayerPattern,

    /// Normalized color calibration: white balance, per-illuminant profiles
    /// and the camera -> XYZ D50 transform.
    pub color: ColorCalibration,
}

impl<T: Primitive + Sync> IPostProcessMetadata<T> for ProcessMetadata<T> {
    fn bayer_pattern(&self) -> BayerPattern { self.bayer_pattern }
    fn color(&self) -> &ColorCalibration { &self.color }
    fn levels(&self) -> &Levels<T> { &self.levels }
}

/// Black and white levels for a single-sample Bayer RAW image.
#[derive(Clone, Debug)]
pub struct Levels<T>
where
    T: Primitive,
{
    /// Repeating black-level pattern from DNG `BlackLevel`, with its repeat
    /// shape given by `BlackLevelRepeatDim`.
    ///
    /// The pattern is applied tile-by-tile from the top-left raw pixel.
    pub black: MonoImage<T>,

    /// Per-column black-level adjustments from DNG `BlackLevelDeltaH`.
    ///
    /// LibRaw folds these deltas into its scalar black level and does not
    /// retain the individual tag values, so decoders using LibRaw leave this
    /// vector empty.
    pub black_delta_h: Vec<T>,

    /// Per-row black-level adjustments from DNG `BlackLevelDeltaV`.
    ///
    /// LibRaw folds these deltas into its scalar black level and does not
    /// retain the individual tag values, so decoders using LibRaw leave this
    /// vector empty.
    pub black_delta_v: Vec<T>,

    /// White level from DNG `WhiteLevel` for single-sample Bayer RAW.
    pub white: T,
}

// ---------------------------------------------------------------------------
// Light sources
// ---------------------------------------------------------------------------

/// EXIF `LightSource` / DNG `CalibrationIlluminant` (SHORT in the file).
///
/// Unknown values are kept in [`LightSource::Reserved`] instead of failing the
/// whole decode.
#[derive(
    Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, FromPrimitive, IntoPrimitive,
)]
#[repr(u16)]
pub enum LightSource {
    Unknown = 0,
    Daylight = 1,
    Fluorescent = 2,
    Tungsten = 3,
    Flash = 4,
    FineWeather = 9,
    CloudyWeather = 10,
    Shade = 11,
    DaylightFluorescent = 12,
    DayWhiteFluorescent = 13,
    CoolWhiteFluorescent = 14,
    WhiteFluorescent = 15,
    StandardLightA = 17,
    StandardLightB = 18,
    StandardLightC = 19,
    D55 = 20,
    D65 = 21,
    D75 = 22,
    D50 = 23,
    ISOStudioTungsten = 24,
    /// Custom illuminant; see [`IlluminantData`].
    Other = 255,
    #[num_enum(catch_all)]
    Reserved(u16),
}

impl LightSource {
    /// Conventional CCT in kelvin. Used for ordering and interpolation weights
    /// only. `None` for `Unknown`, `Other` and reserved values.
    pub fn cct(self) -> Option<f64> {
        use LightSource::*;
        Some(match self {
            Tungsten => 2850.0,
            StandardLightA => 2856.0,
            ISOStudioTungsten => 3200.0,
            WhiteFluorescent => 3500.0,
            CoolWhiteFluorescent => 4000.0,
            Fluorescent => 4200.0,
            StandardLightB => 4874.0,
            DayWhiteFluorescent | D50 => 5000.0,
            Daylight | FineWeather | Flash | D55 => 5500.0,
            DaylightFluorescent => 6400.0,
            CloudyWeather | D65 => 6500.0,
            StandardLightC => 6774.0,
            Shade | D75 => 7500.0,
            Unknown | Other | Reserved(_) => return None,
        })
    }
}

/// Payload of DNG 1.6+ `IlluminantData1/2/3` for `CalibrationIlluminant ==
/// Other`.
///
/// `raw` keeps the original bytes until the encoding is fully parsed;
/// decoders fill `xy` when they can derive a chromaticity.
#[derive(Clone, Debug, PartialEq)]
pub struct IlluminantData {
    pub xy: Option<[f64; 2]>,
    pub raw: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Decoder-facing input (dumb, mirrors the file)
// ---------------------------------------------------------------------------

/// Color metadata exactly as a decoder found it. Everything is optional and no
/// logic lives here. Convert with [`ColorCalibration::new`].
#[derive(Clone, Debug, Default)]
pub struct RawColorMeta {
    /// DNG `AnalogBalance`.
    pub analog_balance: Option<Vec3>,
    /// DNG `AsShotNeutral`: camera response to the scene white.
    pub as_shot_neutral: Option<Vec3>,
    /// DNG `AsShotWhiteXY`.
    pub as_shot_white_xy: Option<[f64; 2]>,
    /// Maker-note style gains `(red, blue)`, green = 1. This is the reciprocal
    /// of a neutral; used only as a fallback.
    pub maker_gains: Option<(f64, f64)>,
    /// Slot-ordered profiles (1..=3), as in the file.
    pub profiles: Vec<RawProfile>,
}

#[derive(Clone, Debug, Default)]
pub struct RawProfile {
    /// `None` = tag missing.
    pub illuminant: Option<LightSource>,
    pub illuminant_data: Option<IlluminantData>,
    pub color_matrix: Option<Matrix3x3>,
    pub forward_matrix: Option<Matrix3x3>,
    pub camera_calibration: Option<Matrix3x3>,
}

// ---------------------------------------------------------------------------
// Normalized model
// ---------------------------------------------------------------------------

/// Where the as-shot white came from (for logging fallbacks).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WhiteSource {
    AsShotNeutral,
    AsShotWhiteXy,
    MakerGains,
    AssumedD65,
}

/// White point request for [`ColorCalibration::resolve`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WhiteSpec {
    /// The normalized as-shot white.
    AsShot,
    /// Camera response to the white (camera-native, post-AB space).
    Neutral(Vec3),
    /// Scene white chromaticity.
    Xy([f64; 2]),
    /// Scene white as a correlated color temperature in kelvin.
    Cct(f64),
}

/// Result of [`ColorCalibration::resolve`].
#[derive(Clone, Debug)]
pub struct ColorTransform {
    /// Neutral normalized to max = 1.
    pub neutral: Vec3,
    /// `1 / neutral`, for white balancing before or after demosaic.
    pub gains: Vec3,
    /// Temperature used to interpolate profiles (kelvin).
    pub cct: f64,
    /// Applies to raw (not white balanced) camera values -> XYZ D50.
    pub cam_to_xyz_d50: Matrix3x3,
    /// Applies to camera values already multiplied by `gains` -> XYZ D50.
    pub cam_to_xyz_d50_post_wb: Matrix3x3,
}

#[derive(Clone, Debug)]
struct Profile {
    /// 1e6 / CCT: sort key and interpolation axis.
    mired: f64,
    /// `AnalogBalance * CameraCalibration`, folded.
    ab_cc: Matrix3x3,
    color_matrix: Option<Matrix3x3>,
    forward_matrix: Option<Matrix3x3>,
}

/// Normalized color calibration. Built once by [`ColorCalibration::new`];
/// the pipeline never handles missing tags.
#[derive(Clone, Debug)]
pub struct ColorCalibration {
    /// 0..=3 entries, sorted by mired ascending, deduplicated.
    profiles: Vec<Profile>,
    /// Decided once: every profile has a ForwardMatrix, use that path.
    use_forward: bool,
    /// Every profile has a ColorMatrix (needed to derive the temperature).
    has_color_matrix: bool,
    as_shot: WhiteSpec,
    as_shot_source: WhiteSource,
}

impl ColorCalibration {
    pub fn new(raw: RawColorMeta) -> Self {
        let n = raw.profiles.len();
        let ab = mat::diag(
            raw.analog_balance
                .filter(|v| valid_positive(v))
                .unwrap_or([1.0; 3]),
        );

        // Profiles: resolve illuminant -> CCT, fold AB*CC, drop empty ones.
        let mut profiles: Vec<Profile> = raw
            .profiles
            .iter()
            .enumerate()
            .filter_map(|(slot, p)| {
                if p.color_matrix.is_none() && p.forward_matrix.is_none() {
                    return None;
                }
                let cct = p
                    .illuminant
                    .and_then(LightSource::cct)
                    .or_else(|| {
                        p.illuminant_data
                            .as_ref()
                            .and_then(|d| d.xy)
                            .map(|[x, y]| xy_to_cct(x, y))
                    })
                    .unwrap_or_else(|| default_cct(slot, n))
                    .clamp(1500.0, 25000.0);
                let cc = p.camera_calibration.unwrap_or(mat::IDENTITY);
                let ab_cc = mat::mul(&ab, &cc);
                // A singular AB*CC cannot be used.
                mat::inv(&ab_cc)?;
                Some(Profile {
                    mired: 1e6 / cct,
                    ab_cc,
                    color_matrix: p.color_matrix,
                    forward_matrix: p.forward_matrix,
                })
            })
            .collect();

        profiles.sort_by(|a, b| a.mired.total_cmp(&b.mired));
        profiles.dedup_by(|cur, prev| (cur.mired - prev.mired).abs() < 1e-3);

        // Choose one path for the whole image; never mix FM and CM.
        let use_forward =
            !profiles.is_empty() && profiles.iter().all(|p| p.forward_matrix.is_some());
        if !use_forward {
            profiles.retain(|p| p.color_matrix.is_some());
        }
        let has_color_matrix =
            !profiles.is_empty() && profiles.iter().all(|p| p.color_matrix.is_some());

        // As-shot white: first valid source wins.
        let (as_shot, as_shot_source) =
            if let Some(nv) = raw.as_shot_neutral.filter(|v| valid_positive(v)) {
                (
                    WhiteSpec::Neutral(normalize_neutral(nv)),
                    WhiteSource::AsShotNeutral,
                )
            } else if let Some(xy) = raw.as_shot_white_xy.filter(|xy| xy[0] > 0.0 && xy[1] > 0.0) {
                (WhiteSpec::Xy(xy), WhiteSource::AsShotWhiteXy)
            } else if let Some((r, b)) = raw.maker_gains.filter(|(r, b)| *r > 0.0 && *b > 0.0) {
                (
                    WhiteSpec::Neutral(normalize_neutral([1.0 / r, 1.0, 1.0 / b])),
                    WhiteSource::MakerGains,
                )
            } else {
                (WhiteSpec::Xy(D65_XY), WhiteSource::AssumedD65)
            };

        Self {
            profiles,
            use_forward,
            has_color_matrix,
            as_shot,
            as_shot_source,
        }
    }

    pub fn as_shot_source(&self) -> WhiteSource { self.as_shot_source }

    /// True when no usable matrices exist and `resolve` falls back to
    /// "camera is sRGB-primaries".
    pub fn is_fallback(&self) -> bool { self.profiles.is_empty() }

    /// Build the transforms for a white point. Cheap enough to call per edit
    /// (as-shot, user CCT, click-to-gray).
    pub fn resolve(&self, white: WhiteSpec) -> ColorTransform {
        let white = match white {
            WhiteSpec::AsShot => self.as_shot,
            w => w,
        };

        if self.profiles.is_empty() {
            return self.resolve_fallback(white);
        }

        // 1. neutral + temperature
        let (neutral, mired) = match white {
            WhiteSpec::AsShot => unreachable!(),
            WhiteSpec::Neutral(n) => {
                let n = normalize_neutral(n);
                let mired = if self.has_color_matrix {
                    self.solve_mired(&n)
                } else {
                    1e6 / 6500.0
                };
                (n, mired)
            }
            WhiteSpec::Cct(t) => {
                let t = t.clamp(1500.0, 25000.0);
                let xy = cct_to_xy(t);
                (self.neutral_from_xy(xy, 1e6 / t), 1e6 / t)
            }
            WhiteSpec::Xy([x, y]) => {
                let t = xy_to_cct(x, y).clamp(1500.0, 25000.0);
                (self.neutral_from_xy([x, y], 1e6 / t), 1e6 / t)
            }
        };

        // 2. camera -> XYZ D50 on raw values
        let ab_cc = self
            .interp(mired, |p| Some(p.ab_cc))
            .unwrap_or(mat::IDENTITY);
        let ab_cc_inv = mat::inv(&ab_cc).unwrap_or(mat::IDENTITY);

        let cam_to_xyz = if self.use_forward {
            let fm = self
                .interp(mired, |p| p.forward_matrix)
                .unwrap_or(mat::IDENTITY);
            let ref_neutral = mat::mul_vec(&ab_cc_inv, &neutral);
            let d = mat::diag(ref_neutral.map(|v| 1.0 / v));
            mat::mul(&mat::mul(&fm, &d), &ab_cc_inv)
        } else {
            let cm = self
                .interp(mired, |p| p.color_matrix)
                .unwrap_or(mat::IDENTITY);
            let xyz_to_cam = mat::mul(&ab_cc, &cm);
            let c2x = mat::inv(&xyz_to_cam).unwrap_or(mat::IDENTITY);
            let src_white = mat::mul_vec(&c2x, &neutral);
            let adapt = bradford(src_white, D50_XYZ);
            let m = mat::mul(&adapt, &c2x);
            // Keep Y(white) = 1.
            let y = mat::mul_vec(&m, &neutral)[1];
            if y.abs() > 1e-12 {
                mat::scale(&m, 1.0 / y)
            } else {
                m
            }
        };

        finish(neutral, 1e6 / mired, cam_to_xyz)
    }

    fn resolve_fallback(&self, white: WhiteSpec) -> ColorTransform {
        let neutral = match white {
            WhiteSpec::Neutral(n) => normalize_neutral(n),
            _ => [1.0; 3],
        };
        let cct = match white {
            WhiteSpec::Cct(t) => t,
            WhiteSpec::Xy([x, y]) => xy_to_cct(x, y),
            _ => 6500.0,
        };
        // Camera treated as sRGB primaries, already balanced by `neutral`.
        let m = mat::mul(&SRGB_TO_XYZ_D50, &mat::diag(neutral.map(|v| 1.0 / v)));
        finish(neutral, cct, m)
    }

    /// Iterate T until the white XYZ implied by `neutral` agrees with the
    /// interpolated matrices. Returns mired.
    fn solve_mired(&self, neutral: &Vec3) -> f64 {
        let mut t = 5000.0_f64;
        for _ in 0..30 {
            let mired = 1e6 / t;
            let ab_cc = self
                .interp(mired, |p| Some(p.ab_cc))
                .unwrap_or(mat::IDENTITY);
            let cm = self
                .interp(mired, |p| p.color_matrix)
                .unwrap_or(mat::IDENTITY);
            let Some(c2x) = mat::inv(&mat::mul(&ab_cc, &cm)) else {
                break;
            };
            let xyz = mat::mul_vec(&c2x, neutral);
            let sum: f64 = xyz.iter().sum();
            if !(sum > 0.0) {
                break;
            }
            let next = xy_to_cct(xyz[0] / sum, xyz[1] / sum).clamp(1500.0, 25000.0);
            let done = (next - t).abs() < 1.0;
            t = next;
            if done {
                break;
            }
        }
        1e6 / t
    }

    /// `neutral = AB*CC*CM(T) * XYZ_white`, normalized.
    ///
    /// With a ForwardMatrix-only file there is no way to derive a neutral from
    /// a chromaticity, so this returns (1,1,1) (no white balance).
    fn neutral_from_xy(&self, xy: [f64; 2], mired: f64) -> Vec3 {
        if !self.has_color_matrix {
            return [1.0; 3];
        }
        let ab_cc = self
            .interp(mired, |p| Some(p.ab_cc))
            .unwrap_or(mat::IDENTITY);
        let cm = self
            .interp(mired, |p| p.color_matrix)
            .unwrap_or(mat::IDENTITY);
        let n = mat::mul_vec(&mat::mul(&ab_cc, &cm), &xy_to_xyz(xy));
        if valid_positive(&n) {
            normalize_neutral(n)
        } else {
            [1.0; 3]
        }
    }

    /// Linear interpolation over mired between the two bracketing profiles,
    /// clamped at the ends. `None` if the selected field is missing.
    fn interp(&self, mired: f64, f: impl Fn(&Profile) -> Option<Matrix3x3>) -> Option<Matrix3x3> {
        let p = &self.profiles;
        let last = p.len().checked_sub(1)?;
        if last == 0 || mired <= p[0].mired {
            return f(&p[0]);
        }
        if mired >= p[last].mired {
            return f(&p[last]);
        }
        let i = p.iter().rposition(|q| q.mired <= mired)?;
        let w = (mired - p[i].mired) / (p[i + 1].mired - p[i].mired);
        Some(mat::lerp(&f(&p[i])?, &f(&p[i + 1])?, w))
    }
}

fn finish(neutral: Vec3, cct: f64, cam_to_xyz_d50: Matrix3x3) -> ColorTransform {
    ColorTransform {
        neutral,
        gains: neutral.map(|v| 1.0 / v),
        cct,
        cam_to_xyz_d50_post_wb: mat::mul(&cam_to_xyz_d50, &mat::diag(neutral)),
        cam_to_xyz_d50,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const D65_XY: [f64; 2] = [0.3127, 0.3290];
const D50_XYZ: Vec3 = [0.96422, 1.0, 0.82521];

/// Linear sRGB (D50-adapted) -> XYZ D50. Used only for the "camera is sRGB"
/// fallback.
const SRGB_TO_XYZ_D50: Matrix3x3 = [
    [0.4360747, 0.3850649, 0.1430804],
    [0.2225045, 0.7168786, 0.0606169],
    [0.0139322, 0.0971045, 0.7141733],
];

/// Default CCT for a profile whose illuminant tag is missing.
fn default_cct(slot: usize, count: usize) -> f64 {
    match (count, slot) {
        (1, _) => 6500.0,
        (_, 0) => 2856.0,
        (_, 1) => 6500.0,
        _ => 5000.0,
    }
}

fn valid_positive(v: &Vec3) -> bool { v.iter().all(|x| x.is_finite() && *x > 0.0) }

fn normalize_neutral(n: Vec3) -> Vec3 {
    let max = n.iter().cloned().fold(f64::MIN, f64::max);
    if max.is_finite() && max > 0.0 && valid_positive(&n) {
        n.map(|v| v / max)
    } else {
        [1.0; 3]
    }
}

fn xy_to_xyz([x, y]: [f64; 2]) -> Vec3 { [x / y, 1.0, (1.0 - x - y) / y] }

/// McCamy's approximation (good to a few K for 2856-6500 K).
pub fn xy_to_cct(x: f64, y: f64) -> f64 {
    let n = (x - 0.3320) / (0.1858 - y);
    449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33
}

/// Kim et al. Planckian locus approximation, 1667-25000 K.
pub fn cct_to_xy(t: f64) -> [f64; 2] {
    let t = t.clamp(1667.0, 25000.0);
    let (t2, t3) = (t * t, t * t * t);
    let x = if t <= 4000.0 {
        -0.2661239e9 / t3 - 0.2343589e6 / t2 + 0.8776956e3 / t + 0.179910
    } else {
        -3.0258469e9 / t3 + 2.1070379e6 / t2 + 0.2226347e3 / t + 0.240390
    };
    let y = if t <= 2222.0 {
        -1.1063814 * x.powi(3) - 1.34811020 * x.powi(2) + 2.18555832 * x - 0.20219683
    } else if t <= 4000.0 {
        -0.9549476 * x.powi(3) - 1.37418593 * x.powi(2) + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x.powi(3) - 5.87338670 * x.powi(2) + 3.75112997 * x - 0.37001483
    };
    [x, y]
}

/// Bradford chromatic adaptation matrix from `src` white to `dst` white (XYZ).
fn bradford(src: Vec3, dst: Vec3) -> Matrix3x3 {
    const M_A: Matrix3x3 = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    let Some(m_a_inv) = mat::inv(&M_A) else {
        return mat::IDENTITY;
    };
    let s = mat::mul_vec(&M_A, &src);
    let d = mat::mul_vec(&M_A, &dst);
    if s.iter().any(|v| v.abs() < 1e-12) {
        return mat::IDENTITY;
    }
    let ratio = [d[0] / s[0], d[1] / s[1], d[2] / s[2]];
    mat::mul(&m_a_inv, &mat::mul(&mat::diag(ratio), &M_A))
}

mod mat {
    use super::{Matrix3x3, Vec3};

    pub const IDENTITY: Matrix3x3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    pub fn mul(a: &Matrix3x3, b: &Matrix3x3) -> Matrix3x3 {
        let mut o = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    o[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        o
    }

    pub fn mul_vec(a: &Matrix3x3, v: &Vec3) -> Vec3 {
        [
            a[0][0] * v[0] + a[0][1] * v[1] + a[0][2] * v[2],
            a[1][0] * v[0] + a[1][1] * v[1] + a[1][2] * v[2],
            a[2][0] * v[0] + a[2][1] * v[1] + a[2][2] * v[2],
        ]
    }

    pub fn diag(v: Vec3) -> Matrix3x3 { [[v[0], 0.0, 0.0], [0.0, v[1], 0.0], [0.0, 0.0, v[2]]] }

    pub fn scale(a: &Matrix3x3, s: f64) -> Matrix3x3 { a.map(|r| r.map(|v| v * s)) }

    pub fn lerp(a: &Matrix3x3, b: &Matrix3x3, w: f64) -> Matrix3x3 {
        let mut o = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                o[i][j] = a[i][j] * (1.0 - w) + b[i][j] * w;
            }
        }
        o
    }

    pub fn inv(m: &Matrix3x3) -> Option<Matrix3x3> {
        let c00 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
        let c01 = m[1][2] * m[2][0] - m[1][0] * m[2][2];
        let c02 = m[1][0] * m[2][1] - m[1][1] * m[2][0];
        let det = m[0][0] * c00 + m[0][1] * c01 + m[0][2] * c02;
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        let id = 1.0 / det;
        Some([
            [
                c00 * id,
                (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * id,
                (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * id,
            ],
            [
                c01 * id,
                (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * id,
                (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * id,
            ],
            [
                c02 * id,
                (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * id,
                (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * id,
            ],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, eps: f64) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < eps)
    }

    // A plausible camera: ColorMatrix = inverse of a mild camera->XYZ.
    fn cm_d65() -> Matrix3x3 {
        mat::inv(&[[0.65, 0.25, 0.06], [0.28, 0.70, 0.02], [0.02, 0.10, 0.75]]).unwrap()
    }

    #[test]
    fn color_matrix_path_maps_neutral_to_d50_white() {
        let cal = ColorCalibration::new(RawColorMeta {
            as_shot_neutral: Some([0.5, 1.0, 0.65]),
            profiles: vec![RawProfile {
                illuminant: Some(LightSource::D65),
                color_matrix: Some(cm_d65()),
                ..Default::default()
            }],
            ..Default::default()
        });
        let t = cal.resolve(WhiteSpec::AsShot);
        let w = mat::mul_vec(&t.cam_to_xyz_d50, &t.neutral);
        assert!(close(w, D50_XYZ, 2e-3), "{w:?}");
        let wb = mat::mul_vec(&t.cam_to_xyz_d50_post_wb, &[1.0; 3]);
        assert!(close(wb, D50_XYZ, 2e-3), "{wb:?}");
    }

    #[test]
    fn forward_matrix_path_sanity() {
        let fm = [[0.70, 0.15, 0.1], [0.30, 0.65, 0.05], [0.02, 0.05, 0.7792]];
        // Rescale rows so FM*[1,1,1] == D50.
        let mut f = fm;
        for i in 0..3 {
            let s: f64 = f[i].iter().sum();
            f[i] = f[i].map(|v| v * D50_XYZ[i] / s);
        }
        let cal = ColorCalibration::new(RawColorMeta {
            as_shot_neutral: Some([0.5, 1.0, 0.65]),
            profiles: vec![RawProfile {
                illuminant: Some(LightSource::D65),
                color_matrix: Some(cm_d65()),
                forward_matrix: Some(f),
                ..Default::default()
            }],
            ..Default::default()
        });
        let t = cal.resolve(WhiteSpec::AsShot);
        let w = mat::mul_vec(&t.cam_to_xyz_d50_post_wb, &[1.0; 3]);
        assert!(close(w, D50_XYZ, 1e-6), "{w:?}");
    }

    #[test]
    fn missing_everything_falls_back() {
        let cal = ColorCalibration::new(RawColorMeta::default());
        assert!(cal.is_fallback());
        assert_eq!(cal.as_shot_source(), WhiteSource::AssumedD65);
        let t = cal.resolve(WhiteSpec::AsShot);
        let w = mat::mul_vec(&t.cam_to_xyz_d50_post_wb, &[1.0; 3]);
        assert!(close(w, D50_XYZ, 2e-3));
    }

    #[test]
    fn two_profiles_sorted_and_interpolated() {
        let cal = ColorCalibration::new(RawColorMeta {
            maker_gains: Some((2.0, 1.5)),
            // Slot order reversed on purpose; no illuminant tags -> A, D65
            // defaults.
            profiles: vec![
                RawProfile {
                    color_matrix: Some(cm_d65()),
                    ..Default::default()
                },
                RawProfile {
                    color_matrix: Some(cm_d65()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        assert_eq!(cal.as_shot_source(), WhiteSource::MakerGains);
        let t = cal.resolve(WhiteSpec::Cct(4500.0));
        assert!(t.cct > 4000.0 && t.cct < 5000.0);
    }

    #[test]
    fn light_source_roundtrip() {
        assert_eq!(LightSource::from(21u16), LightSource::D65);
        assert_eq!(LightSource::from(999u16), LightSource::Reserved(999));
        assert_eq!(u16::from(LightSource::Reserved(999)), 999);
    }
}
