use std::fmt::Display;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::{Levels, Metadata, WhiteBalance};
use fornax_core::protocol::IDecoder;
use fornax_core::{DecodedRawActive, DecodedRawFull, Primitive};
use generic_num::num;
use image::math::Rect;

use crate::Libraw;

pub struct LibrawDecoder;

fn backend_err<E>(error: E) -> DecodeError
where
    E: Display,
{
    DecodeError::Backend(error.to_string().into())
}

impl<T: Primitive> IDecoder<Path, T> for LibrawDecoder {
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_file(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_full().map_err(backend_err),
        )
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_file(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_active().map_err(backend_err),
        )
    }
}

impl<T: Primitive> IDecoder<[u8], T> for LibrawDecoder {
    fn decode_full(&self, input: &[u8]) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_buffer(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_full().map_err(backend_err),
        )
    }

    fn decode_active(
        &self,
        input: &[u8],
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_buffer(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_active().map_err(backend_err),
        )
    }
}

fn decode<T, R>(
    open: impl FnOnce(&Libraw) -> Result<(), DecodeError>,
    extract: impl FnOnce(&Libraw) -> Result<R, DecodeError>,
) -> Result<(R, Metadata<T>), DecodeError>
where
    T: Primitive,
{
    let libraw = Libraw::default();
    open(&libraw)?;
    let image = extract(&libraw)?;
    Ok((image, get_metadata(&libraw)?))
}

fn get_metadata<T>(libraw: &Libraw) -> Result<Metadata<T>, DecodeError>
where
    T: Primitive,
{
    let imgdata = libraw.arc_imgdata_ptr();
    let sizes = &imgdata.rawdata.sizes;
    let color = &imgdata.color;
    let dng = &color.dng_levels;

    // dbg!(color.WBCT_Coeffs);

    let bayer_pattern = libraw
        .bayer_pattern()
        .map_err(|error| backend_err(format!("reading LibRaw CFA metadata: {error}")))?;

    let active_area = {
        let active_area = Rect {
            x: sizes.left_margin as u32,
            y: sizes.top_margin as u32,
            width: sizes.width as u32,
            height: sizes.height as u32,
        };
        if active_area.width == 0
            || active_area.height == 0
            || active_area.x.saturating_add(active_area.width) > sizes.raw_width as u32
            || active_area.y.saturating_add(active_area.height) > sizes.raw_height as u32
        {
            return Err(backend_err(format!(
                "invalid LibRaw active area {}x{} at ({}, {}) for {}x{} raw image",
                active_area.width,
                active_area.height,
                active_area.x,
                active_area.y,
                sizes.raw_width,
                sizes.raw_height
            )));
        }
        active_area
    };

    let default_crop = {
        #[allow(clippy::unnecessary_cast, reason = "bindgen generate i32 on windows")]
        let crop_origin_present = dng.parsedfields
            & libraw_sys::LibRaw_dngfields_marks_LIBRAW_DNGFM_CROPORIGIN as u32
            != 0;
        #[allow(clippy::unnecessary_cast, reason = "bindgen generate i32 on windows")]
        let crop_size_present =
            dng.parsedfields & libraw_sys::LibRaw_dngfields_marks_LIBRAW_DNGFM_CROPSIZE as u32 != 0;
        let dng_crop = Rect {
            x: dng.default_crop[0] as u32,
            y: dng.default_crop[1] as u32,
            width: dng.default_crop[2] as u32,
            height: dng.default_crop[3] as u32,
        };
        let dng_crop_valid = crop_origin_present
            && crop_size_present
            && dng_crop.width > 0
            && dng_crop.height > 0
            && dng_crop.x.saturating_add(dng_crop.width) <= active_area.width
            && dng_crop.y.saturating_add(dng_crop.height) <= active_area.height;
        let raw_inset = sizes.raw_inset_crops[0];
        let promoted_dng_crop = raw_inset
            .cleft
            .checked_sub(sizes.left_margin)
            .and_then(|x| {
                raw_inset.ctop.checked_sub(sizes.top_margin).map(|y| Rect {
                    x: x as u32,
                    y: y as u32,
                    width: raw_inset.cwidth as u32,
                    height: raw_inset.cheight as u32,
                })
            });
        let promoted_dng_crop = promoted_dng_crop.filter(|crop| {
            imgdata.idata.dng_version != 0
                && crop.width > 0
                && crop.height > 0
                && crop.x.saturating_add(crop.width) <= active_area.width
                && crop.y.saturating_add(crop.height) <= active_area.height
        });
        if dng_crop_valid {
            dng_crop
        } else if let Some(crop) = promoted_dng_crop {
            crop
        } else {
            Rect {
                x: 0,
                y: 0,
                width: active_area.width,
                height: active_area.height,
            }
        }
    };

    let black = {
        let dng_rows = dng.dng_cblack[4] as usize;
        let dng_cols = dng.dng_cblack[5] as usize;
        let color_rows = color.cblack[4] as usize;
        let color_cols = color.cblack[5] as usize;
        let (width, height, values): (usize, usize, Vec<f32>) = if dng_rows > 0
            && dng_cols > 0
            && dng_rows.saturating_mul(dng_cols) <= 4096
        {
            let values = dng.dng_fcblack[6..6 + dng_rows * dng_cols]
                .iter()
                .map(|black| dng.dng_fblack + black)
                .collect();
            (dng_cols, dng_rows, values)
        } else if color_rows > 0 && color_cols > 0 && color_rows.saturating_mul(color_cols) <= 4096
        {
            let values = color.cblack[6..6 + color_rows * color_cols]
                .iter()
                .map(|black| color.black as f32 + *black as f32)
                .collect();
            (color_cols, color_rows, values)
        } else {
            let values = bayer_pattern
                .as_mask()
                .iter()
                .map(|channel| color.black as f32 + color.cblack[channel.index()] as f32)
                .collect();
            (2, 2, values)
        };
        image::ImageBuffer::from_vec(
            width as u32,
            height as u32,
            values
                .into_iter()
                .map(|value| num!(value))
                .collect::<Vec<T>>(),
        )
        .ok_or_else(|| {
            backend_err(format!(
                "LibRaw black-level pattern is inconsistent with {width}x{height} dimensions"
            ))
        })?
    };

    let white = {
        #[allow(clippy::unnecessary_cast, reason = "bindgen generate i32 on windows")]
        let white_present =
            dng.parsedfields & libraw_sys::LibRaw_dngfields_marks_LIBRAW_DNGFM_WHITE as u32 != 0;
        let white = if white_present && dng.dng_whitelevel[0] > 0 {
            dng.dng_whitelevel[0]
        } else {
            color.maximum
        };
        if white == 0 {
            return Err(backend_err(
                "LibRaw reported a zero white level for a single-sample Bayer image",
            ));
        }
        white
    };
    let as_shot_neutral = {
        if color.cam_mul[0] > 0.0 && color.cam_mul[1] > 0.0 && color.cam_mul[2] > 0.0 {
            Some(WhiteBalance::CamMul(
                num!(color.cam_mul[0]),
                num!(color.cam_mul[1]),
                num!(color.cam_mul[2]),
            ))
        } else if dng.asshotneutral[0] > 0.0
            && dng.asshotneutral[1] > 0.0
            && dng.asshotneutral[2] > 0.0
        {
            Some(WhiteBalance::AsShotNeutral(
                num!(dng.asshotneutral[0]),
                num!(dng.asshotneutral[1]),
                num!(dng.asshotneutral[2]),
            ))
        } else {
            None
        }
    };
    Ok(Metadata {
        active_area,
        default_crop,
        levels: Levels {
            black,
            black_delta_h: Vec::new(),
            black_delta_v: Vec::new(),
            white: num!(white),
        },
        bayer_pattern,
        white_balance: as_shot_neutral,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use fornax_core::BayerPattern;
    use image::math::Rect;

    use super::*;

    #[test]
    fn cr2_metadata_matches_active_image() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;

        let metadata = get_metadata::<f32>(&libraw)?;
        assert_eq!(
            metadata.active_area,
            Rect {
                x: 158,
                y: 52,
                width: 5_202,
                height: 3_464,
            }
        );
        assert_eq!(
            metadata.default_crop,
            Rect {
                x: 0,
                y: 0,
                width: 5_202,
                height: 3_464,
            }
        );
        assert_eq!(metadata.levels.black.dimensions(), (2, 2));
        assert!(
            metadata
                .levels
                .black
                .as_raw()
                .iter()
                .all(|&black| black >= 0.0 && black < metadata.levels.white)
        );
        assert_eq!(metadata.levels.white, 13_584.0);
        assert_eq!(metadata.bayer_pattern, BayerPattern::GBRG);
        assert!(metadata.levels.black_delta_h.is_empty());
        assert!(metadata.levels.black_delta_v.is_empty());
        Ok(())
    }

    #[test]
    fn dng_metadata_uses_default_crop_and_repeat_black() -> mischief::Result<()> {
        let path =
            fornax_devtool::root_dir().join("external/raw-images/images/colorchart-5D2-6000K.dng");
        let libraw = Libraw::default();
        libraw.open_file(&path)?.unpack()?;

        let metadata = get_metadata::<f32>(&libraw)?;
        assert_eq!(
            metadata.active_area,
            Rect {
                x: 158,
                y: 52,
                width: 5_634,
                height: 3_752,
            }
        );
        assert_eq!(
            metadata.default_crop,
            Rect {
                x: 10,
                y: 4,
                width: 5_616,
                height: 3_744,
            }
        );
        assert_eq!(metadata.levels.black.dimensions(), (2, 2));
        assert!(
            metadata
                .levels
                .black
                .as_raw()
                .iter()
                .all(|&black| black == 1_024.0)
        );
        assert_eq!(metadata.levels.white, 15_600.0);
        assert_eq!(metadata.bayer_pattern, BayerPattern::GBRG);
        Ok(())
    }

    #[test]
    fn metadata_wrapper_keeps_libraw_alive() -> mischief::Result<()> {
        let iparams = {
            let libraw = Libraw::default();
            libraw
                .open_file(&fornax_devtool::raw_file())?
                .unpack()?
                .get_iparams()?
        };

        assert_eq!(iparams.make()?, "Canon");
        Ok(())
    }

    #[test]
    fn missing_file_returns_nonempty_backend_error() {
        let result: Result<(DecodedRawActive<f32>, Metadata<f32>), DecodeError> =
            LibrawDecoder.decode_active(Path::new("this-file-does-not-exist.cr2"));

        let error = result.expect_err("missing file should fail");
        assert!(matches!(&error, DecodeError::Backend(_)));
        let message = error.to_string();
        assert!(message.len() > "decoder backend failure: ".len());
    }
}
