use fornax_core::{BayerChannel, BayerImage, FornaxPrimitive};
use image::ImageBuffer;
use rayon::prelude::*;
fn get_diagnal_value<T>(img: &ImageBuffer<image::Luma<T>, Vec<T>>, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let top_left = img.get_pixel(x - 1, y - 1);
    let top_right = img.get_pixel(x + 1, y - 1);
    let bottom_left = img.get_pixel(x - 1, y + 1);
    let bottom_right = img.get_pixel(x + 1, y + 1);
    (top_left[0] + top_right[0] + bottom_left[0] + bottom_right[0]) / T::from(4).unwrap()
}

fn get_neighbour_value<T>(img: &ImageBuffer<image::Luma<T>, Vec<T>>, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let left = img.get_pixel(x - 1, y);
    let right = img.get_pixel(x + 1, y);
    let top = img.get_pixel(x, y - 1);
    let buttom = img.get_pixel(x, y + 1);
    (left[0] + right[0] + top[0] + buttom[0]) / T::from(4).unwrap()
}

fn get_left_right_value<T>(img: &ImageBuffer<image::Luma<T>, Vec<T>>, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let left = img.get_pixel(x - 1, y);
    let right = img.get_pixel(x + 1, y);

    (left[0] + right[0]) / T::from(2).unwrap()
}

fn get_top_down_value<T>(img: &ImageBuffer<image::Luma<T>, Vec<T>>, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let top = img.get_pixel(x, y - 1);
    let buttom = img.get_pixel(x, y + 1);
    (top[0] + buttom[0]) / T::from(2).unwrap()
}
fn get_diagnal_value_check<T>(
    img: &ImageBuffer<image::Luma<T>, Vec<T>>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0;
    let top_left = if x != 0 && y != 0 {
        count += 1;
        img.get_pixel(x - 1, y - 1)[0]
    } else {
        T::from(0).unwrap()
    };
    let top_right = if x < width - 1 && y != 0 {
        count += 1;
        img.get_pixel(x + 1, y - 1)[0]
    } else {
        T::from(0).unwrap()
    };
    let bottom_left = if x != 0 && y < height - 1 {
        count += 1;
        img.get_pixel(x - 1, y + 1)[0]
    } else {
        T::from(0).unwrap()
    };
    let bottom_right = if x < width - 1 && y < height - 1 {
        count += 1;
        img.get_pixel(x + 1, y + 1)[0]
    } else {
        T::from(0).unwrap()
    };
    (top_left + top_right + bottom_left + bottom_right) / T::from(count).unwrap()
}

fn get_neighbour_value_check<T>(
    img: &ImageBuffer<image::Luma<T>, Vec<T>>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0;
    let left = if x != 0 {
        count += 1;
        img.get_pixel(x - 1, y)[0]
    } else {
        T::from(0).unwrap()
    };
    let right = if x < width - 1 {
        count += 1;
        img.get_pixel(x + 1, y)[0]
    } else {
        T::from(0).unwrap()
    };
    let top = if y != 0 {
        count += 1;
        img.get_pixel(x, y - 1)[0]
    } else {
        T::from(0).unwrap()
    };

    let buttom = if y < height - 1 {
        count += 1;
        img.get_pixel(x, y + 1)[0]
    } else {
        T::from(0).unwrap()
    };
    (left + right + top + buttom) / T::from(count).unwrap()
}

fn get_left_right_value_check<T>(
    img: &ImageBuffer<image::Luma<T>, Vec<T>>,
    x: u32,
    y: u32,
    width: u32,
) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0;
    let left = if x != 0 {
        count += 1;
        img.get_pixel(x - 1, y)[0]
    } else {
        T::from(0).unwrap()
    };
    let right = if x < width - 1 {
        count += 1;
        img.get_pixel(x + 1, y)[0]
    } else {
        T::from(0).unwrap()
    };

    (left + right) / T::from(count).unwrap()
}

fn get_top_down_value_check<T>(
    img: &ImageBuffer<image::Luma<T>, Vec<T>>,
    x: u32,
    y: u32,
    height: u32,
) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0;

    let top = if y != 0 {
        count += 1;
        img.get_pixel(x, y - 1)[0]
    } else {
        T::from(0).unwrap()
    };

    let buttom = if y < height - 1 {
        count += 1;
        img.get_pixel(x, y + 1)[0]
    } else {
        T::from(0).unwrap()
    };
    (top + buttom) / T::from(count).unwrap()
}
pub struct DemosaicLinear();
impl<T> super::IDemosaic<T> for DemosaicLinear
where
    T: FornaxPrimitive,
{
    fn demosaic(&self, bayer_image: &BayerImage<T>) -> ImageBuffer<image::Rgb<T>, Vec<T>> {
        let mosaic: &ImageBuffer<image::Luma<T>, Vec<T>> = bayer_image.mosaic();
        let pattern = bayer_image.pattern();
        let (width, height) = mosaic.dimensions();
        let mut img: ImageBuffer<image::Rgb<T>, Vec<T>> = ImageBuffer::new(width, height);
        let bayer_mask = pattern.as_mask();
        clerk::debug!("Start demosaicing.");
        img.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
            // `(*x & 1) + 2 * (*y & 1)` is the of the current pixel at image (x,y) index in
            // bayer pattern.
            match (
                &bayer_mask[((x & 1) + 2 * (y & 1)) as usize],
                x > 0 && y > 0 && x < width - 1 && y < height - 1,
            ) {
                (BayerChannel::R, true) => {
                    pixel[0] = mosaic.get_pixel(x, y)[0];
                    pixel[1] = get_neighbour_value(mosaic, x, y);
                    pixel[2] = get_diagnal_value(mosaic, x, y);
                }
                (BayerChannel::G, true) => {
                    pixel[0] = get_left_right_value(mosaic, x, y);
                    pixel[1] = mosaic.get_pixel(x, y)[0];
                    pixel[2] = get_top_down_value(mosaic, x, y);
                }
                (BayerChannel::B, true) => {
                    pixel[0] = get_diagnal_value(mosaic, x, y);
                    pixel[1] = get_neighbour_value(mosaic, x, y);
                    pixel[2] = mosaic.get_pixel(x, y)[0];
                }
                (BayerChannel::G2, true) => {
                    pixel[0] = get_top_down_value(mosaic, x, y);
                    pixel[1] = mosaic.get_pixel(x, y)[0];
                    pixel[2] = get_left_right_value(mosaic, x, y);
                }
                (BayerChannel::R, false) => {
                    pixel[0] = mosaic.get_pixel(x, y)[0];
                    pixel[1] = get_neighbour_value_check(mosaic, x, y, width, height);
                    pixel[2] = get_diagnal_value_check(mosaic, x, y, width, height);
                }
                (BayerChannel::G, false) => {
                    pixel[0] = get_left_right_value_check(mosaic, x, y, width);
                    pixel[1] = mosaic.get_pixel(x, y)[0];
                    pixel[2] = get_top_down_value_check(mosaic, x, y, height);
                }
                (BayerChannel::B, false) => {
                    pixel[0] = get_diagnal_value_check(mosaic, x, y, width, height);
                    pixel[1] = get_neighbour_value_check(mosaic, x, y, width, height);
                    pixel[2] = mosaic.get_pixel(x, y)[0];
                }
                (BayerChannel::G2, false) => {
                    pixel[0] = get_top_down_value_check(mosaic, x, y, height);
                    pixel[1] = mosaic.get_pixel(x, y)[0];
                    pixel[2] = get_left_right_value_check(mosaic, x, y, width);
                }
            }
        });
        clerk::debug!("End demosaicing.");
        img
    }
}
#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    use super::*;
    use crate::demosaic::IDemosaic;
    #[test]
    fn test_linear() {
        tracing_subscriber::registry()
            .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
            .init();
        let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
        let mut img_path = root.clone();
        img_path.push("temp/bayerimga.tiff");
        let bayer = image::ImageReader::open(img_path)
            .unwrap()
            .decode()
            .unwrap()
            .to_luma16();
        let demosaicer = DemosaicLinear();
        let img: ImageBuffer<image::Rgb<u16>, Vec<u16>> = demosaicer.demosaic(
            &fornax_core::BayerImage::new(bayer, fornax_core::BayerPattern::GBRG),
        );
        image::DynamicImage::from(img).save("a.tiff").unwrap();
    }
}
