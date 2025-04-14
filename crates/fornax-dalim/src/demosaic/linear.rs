use fornax_core::{BayerChannel, BayerImage, FornaxBayerImage};
use image::ImageBuffer;
use rayon::prelude::*;
fn get_diagnal_value(img: &FornaxBayerImage, x: u32, y: u32) -> u16 {
    let top_left = img.get_pixel(x - 1, y - 1);
    let top_right = img.get_pixel(x + 1, y - 1);
    let bottom_left = img.get_pixel(x - 1, y + 1);
    let bottom_right = img.get_pixel(x + 1, y + 1);
    (top_left[0] + top_right[0] + bottom_left[0] + bottom_right[0]) / 4
}
fn get_neighbour_value(img: &FornaxBayerImage, x: u32, y: u32) -> u16 {
    let left = img.get_pixel(x - 1, y);
    let right = img.get_pixel(x + 1, y);
    let top = img.get_pixel(x, y - 1);
    let buttom = img.get_pixel(x, y + 1);
    (left[0] + right[0] + top[0] + buttom[0]) / 4
}
fn get_left_right_value(img: &FornaxBayerImage, x: u32, y: u32) -> u16 {
    let left = img.get_pixel(x - 1, y);
    let right = img.get_pixel(x + 1, y);

    (left[0] + right[0]) / 2
}
fn get_top_down_value(img: &FornaxBayerImage, x: u32, y: u32) -> u16 {
    let top = img.get_pixel(x, y - 1);
    let buttom = img.get_pixel(x, y + 1);
    (top[0] + buttom[0]) / 2
}
pub struct DemosaicLinear();
impl super::IDemosaic for DemosaicLinear {
    fn demosaic(bayer_image: &BayerImage) -> ImageBuffer<image::Rgb<u16>, Vec<u16>> {
        let mosaic = bayer_image.mosaic();
        let pattern = bayer_image.pattern();
        let (width, height) = mosaic.dimensions();
        let mut img: ImageBuffer<image::Rgb<u16>, Vec<u16>> = ImageBuffer::new(width, height);
        let bayer_mask = pattern.as_mask();
        img.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
            if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
                // `(*x & 1) + 2 * (*y & 1)` is the of the current pixel at image (x,y) index in
                // bayer pattern.
                match &bayer_mask[((x & 1) + 2 * (y & 1)) as usize] {
                    BayerChannel::R => {
                        pixel[0] = mosaic.get_pixel(x, y)[0];
                        pixel[1] = get_neighbour_value(mosaic, x, y);
                        pixel[2] = get_diagnal_value(mosaic, x, y);
                    }
                    BayerChannel::G => {
                        pixel[0] = get_left_right_value(mosaic, x, y);
                        pixel[1] = mosaic.get_pixel(x, y)[0];
                        pixel[2] = get_top_down_value(mosaic, x, y);
                    }
                    BayerChannel::B => {
                        pixel[0] = get_diagnal_value(mosaic, x, y);
                        pixel[1] = get_neighbour_value(mosaic, x, y);
                        pixel[2] = mosaic.get_pixel(x, y)[0];
                    }
                    BayerChannel::G2 => {
                        pixel[0] = get_top_down_value(mosaic, x, y);
                        pixel[1] = mosaic.get_pixel(x, y)[0];
                        pixel[2] = get_left_right_value(mosaic, x, y);
                    }
                }
            }
        });
        img
    }
}
#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;
    use crate::demosaic::IDemosaic;
    #[test]
    fn test_linear() {
        let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
        let mut img_path = root.clone();
        img_path.push("temp/bayerimga.tiff");
        let bayer = image::ImageReader::open(img_path)
            .unwrap()
            .decode()
            .unwrap()
            .to_luma16();
        let img = DemosaicLinear::demosaic(&fornax_core::BayerImage::new(
            bayer,
            fornax_core::BayerPattern::GBRG,
        ));
        image::DynamicImage::from(img).save("a.tiff").unwrap();
    }
}
