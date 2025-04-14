// use rayon::prelude::*;
// fn get_red_value(
//     img: &image::ImageBuffer<image::Rgba<u16>, Vec<u16>>,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> u16 {
//     if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
//         let top_left = &img.get_pixel(x - 1, y - 1);
//         let top_right = &img.get_pixel(x + 1, y - 1);
//         let bottom_left = &img.get_pixel(x - 1, y + 1);
//         let bottom_right = &img.get_pixel(x + 1, y + 1);
//         (top_left[0] + top_right[0] + bottom_left[0] + bottom_right[0]) / 4
//     } else {
//         img.get_pixel(x, y)[0]
//     }
// }
// fn get_green_value(
//     img: &image::ImageBuffer<image::Rgba<u16>, Vec<u16>>,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> u16 {
//     if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
//         let left = &img.get_pixel(x - 1, y);
//         let right = &img.get_pixel(x + 1, y);
//         let top = &img.get_pixel(x, y - 1);
//         let buttom = &img.get_pixel(x, y + 1);
//         (left[1] + right[1] + top[1] + buttom[1] + left[3] + right[3] + top[3] + buttom[3]) / 4
//     } else {
//         img.get_pixel(x, y)[0]
//     }
// }
// fn get_blue_value(
//     img: &image::ImageBuffer<image::Rgba<u16>, Vec<u16>>,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> u16 {
//     if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
//         let top_left = &img.get_pixel(x - 1, y - 1);
//         let top_right = &img.get_pixel(x + 1, y - 1);
//         let bottom_left = &img.get_pixel(x - 1, y + 1);
//         let bottom_right = &img.get_pixel(x + 1, y + 1);
//         (top_left[2] + top_right[2] + bottom_left[2] + bottom_right[2]) / 4
//     } else {
//         img.get_pixel(x, y)[0]
//     }
// }
// fn get_neighbour_red_blue(
//     img: &image::ImageBuffer<image::Rgba<u16>, Vec<u16>>,
//     x: u32,
//     y: u32,
//     width: u32,
//     height: u32,
// ) -> (u16, u16) {
//     if x > 0 && y > 0 && x < width - 1 && y < height - 1 {
//         let left = &img.get_pixel(x - 1, y);
//         let right = &img.get_pixel(x + 1, y);
//         let top = &img.get_pixel(x, y - 1);
//         let buttom = &img.get_pixel(x, y + 1);
//         let red = (left[0] + right[0] + top[0] + buttom[0]) / 2;
//         let blue = (left[2] + right[2] + top[2] + buttom[2]) / 2;
//         (red, blue)
//     } else {
//         (img.get_pixel(x, y)[0], img.get_pixel(x, y)[0])
//     }
// }
// pub struct DemosaicLinear();
// impl DemosaicLinear {
//     fn demosaic(img: fornax_core::FornaxRawImage) -> image::ImageBuffer<image::Rgb<u16>,
// Vec<u16>> {         let temp_img = img.clone();
//         let (width, height) = temp_img.dimensions();
//         let mut img: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
//             image::ImageBuffer::new(width, height);
//         img.enumerate_pixels_mut()
//             .collect::<Vec<(u32, u32, &mut image::Rgb<u16>)>>()
//             .par_iter_mut()
//             .for_each(|(x, y, pixel)| {
//                 //red
//                 if (*x % 2 == 0) && (*y % 2 == 1) {
//                     pixel[0] = temp_img.get_pixel(*x, *y)[0];
//                     pixel[1] = get_green_value(&temp_img, *x, *y, width, height);
//                     pixel[2] = get_blue_value(&temp_img, *x, *y, width, height);
//                 }
//                 //blue
//                 else if (*x % 2 == 1) && (*y % 2 == 0) {
//                     pixel[0] = get_red_value(&temp_img, *x, *y, width, height);
//                     pixel[1] = get_green_value(&temp_img, *x, *y, width, height);
//                     pixel[2] = temp_img.get_pixel(*x, *y)[2];
//                 }
//                 // green
//                 else {
//                     (pixel[0], pixel[2]) = get_neighbour_red_blue(&temp_img, *x, *y, width,
// height);                     let center = temp_img.get_pixel(*x, *y);
//                     pixel[1] = center[1] + center[3];
//                 }
//             });
//         img
//     }
// }
// #[cfg(test)]
// mod test {
//     use std::path::PathBuf;

//     use super::*;
//     #[test]
//     fn test_linear() {
//         let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
//         let mut img_path = root.clone();
//         img_path.push("temp/raw_image.tiff");
//         let img = image::ImageReader::open(img_path)
//             .unwrap()
//             .decode()
//             .unwrap()
//             .to_rgba16();
//         let dema = DemosaicLinear::demosaic(img);
//         image::DynamicImage::from(dema).save("a.tiff").unwrap();
//     }
// }
