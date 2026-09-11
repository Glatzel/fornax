use fornax_core::{BayerChannel, BayerImage, FornaxPrimitive};
use image::{ImageBuffer, Luma, Rgb};
use rayon::prelude::*;

#[inline(always)]
const fn idx(width: u32, x: u32, y: u32) -> usize { y as usize * width as usize + x as usize }

// ---------- Fast path: caller guarantees (x-1, y-1)..=(x+1, y+1) is in bounds
// ----------

/// # Safety
/// Caller must guarantee `x` is in `1..width-1` and `y` is in `1..height-1`.
#[inline(always)]
unsafe fn diagonal_fast<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    unsafe {
        let tl = *raw.get_unchecked(idx(width, x - 1, y - 1));
        let tr = *raw.get_unchecked(idx(width, x + 1, y - 1));
        let bl = *raw.get_unchecked(idx(width, x - 1, y + 1));
        let br = *raw.get_unchecked(idx(width, x + 1, y + 1));

        (tl + tr + bl + br) / T::from(4).unwrap()
    }
}

/// # Safety
/// Caller must guarantee `x` is in `1..width-1` and `y` is in `1..height-1`.
#[inline(always)]
unsafe fn neighbour_fast<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    unsafe {
        let l = *raw.get_unchecked(idx(width, x - 1, y));
        let r = *raw.get_unchecked(idx(width, x + 1, y));
        let t = *raw.get_unchecked(idx(width, x, y - 1));
        let b = *raw.get_unchecked(idx(width, x, y + 1));
        (l + r + t + b) / T::from(4).unwrap()
    }
}

/// # Safety
/// Caller must guarantee `x` is in `1..width-1`.
#[inline(always)]
unsafe fn left_right_fast<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    unsafe {
        let l = *raw.get_unchecked(idx(width, x - 1, y));
        let r = *raw.get_unchecked(idx(width, x + 1, y));
        (l + r) / T::from(2).unwrap()
    }
}

/// # Safety
/// Caller must guarantee `y` is in `1..height-1`.
#[inline(always)]
unsafe fn top_down_fast<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    unsafe {
        let t = *raw.get_unchecked(idx(width, x, y - 1));
        let b = *raw.get_unchecked(idx(width, x, y + 1));
        (t + b) / T::from(2).unwrap()
    }
}

// ---------- Checked path: used only for border rows/columns ----------

fn diagonal_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0_u32;
    let mut sum = T::from(0).unwrap();
    if x > 0 && y > 0 {
        sum = sum + raw[idx(width, x - 1, y - 1)];
        count += 1;
    }
    if x + 1 < width && y > 0 {
        sum = sum + raw[idx(width, x + 1, y - 1)];
        count += 1;
    }
    if x > 0 && y + 1 < height {
        sum = sum + raw[idx(width, x - 1, y + 1)];
        count += 1;
    }
    if x + 1 < width && y + 1 < height {
        sum = sum + raw[idx(width, x + 1, y + 1)];
        count += 1;
    }
    sum / T::from(count).unwrap()
}

fn neighbour_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0_u32;
    let mut sum = T::from(0).unwrap();
    if x > 0 {
        sum = sum + raw[idx(width, x - 1, y)];
        count += 1;
    }
    if x + 1 < width {
        sum = sum + raw[idx(width, x + 1, y)];
        count += 1;
    }
    if y > 0 {
        sum = sum + raw[idx(width, x, y - 1)];
        count += 1;
    }
    if y + 1 < height {
        sum = sum + raw[idx(width, x, y + 1)];
        count += 1;
    }
    sum / T::from(count).unwrap()
}

fn left_right_checked<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0_u32;
    let mut sum = T::from(0).unwrap();
    if x > 0 {
        sum = sum + raw[idx(width, x - 1, y)];
        count += 1;
    }
    if x + 1 < width {
        sum = sum + raw[idx(width, x + 1, y)];
        count += 1;
    }
    sum / T::from(count).unwrap()
}

fn top_down_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: FornaxPrimitive,
{
    let mut count = 0_u32;
    let mut sum = T::from(0).unwrap();
    if y > 0 {
        sum = sum + raw[idx(width, x, y - 1)];
        count += 1;
    }
    if y + 1 < height {
        sum = sum + raw[idx(width, x, y + 1)];
        count += 1;
    }
    sum / T::from(count).unwrap()
}

// ---------- Per-pixel dispatch ----------

#[inline(always)]
fn write_pixel_fast<T>(
    raw_mosaic: &[T],
    width: u32,
    x: u32,
    y: u32,
    channel: BayerChannel,
    out_row: &mut [T],
) where
    T: FornaxPrimitive,
{
    let o = x as usize * 3;
    // SAFETY: caller only invokes this for x in 1..width-1, y in 1..height-1.
    unsafe {
        let centre = *raw_mosaic.get_unchecked(idx(width, x, y));
        match channel {
            BayerChannel::R => {
                out_row[o] = centre;
                out_row[o + 1] = neighbour_fast(raw_mosaic, width, x, y);
                out_row[o + 2] = diagonal_fast(raw_mosaic, width, x, y);
            }
            BayerChannel::G => {
                out_row[o] = left_right_fast(raw_mosaic, width, x, y);
                out_row[o + 1] = centre;
                out_row[o + 2] = top_down_fast(raw_mosaic, width, x, y);
            }
            BayerChannel::B => {
                out_row[o] = diagonal_fast(raw_mosaic, width, x, y);
                out_row[o + 1] = neighbour_fast(raw_mosaic, width, x, y);
                out_row[o + 2] = centre;
            }
            BayerChannel::G2 => {
                out_row[o] = top_down_fast(raw_mosaic, width, x, y);
                out_row[o + 1] = centre;
                out_row[o + 2] = left_right_fast(raw_mosaic, width, x, y);
            }
        }
    }
}

#[inline(always)]
fn write_pixel_checked<T>(
    raw_mosaic: &[T],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    channel: BayerChannel,
    out_row: &mut [T],
) where
    T: FornaxPrimitive,
{
    let o = x as usize * 3;
    let centre = raw_mosaic[idx(width, x, y)];
    match channel {
        BayerChannel::R => {
            out_row[o] = centre;
            out_row[o + 1] = neighbour_checked(raw_mosaic, width, height, x, y);
            out_row[o + 2] = diagonal_checked(raw_mosaic, width, height, x, y);
        }
        BayerChannel::G => {
            out_row[o] = left_right_checked(raw_mosaic, width, x, y);
            out_row[o + 1] = centre;
            out_row[o + 2] = top_down_checked(raw_mosaic, width, height, x, y);
        }
        BayerChannel::B => {
            out_row[o] = diagonal_checked(raw_mosaic, width, height, x, y);
            out_row[o + 1] = neighbour_checked(raw_mosaic, width, height, x, y);
            out_row[o + 2] = centre;
        }
        BayerChannel::G2 => {
            out_row[o] = top_down_checked(raw_mosaic, width, height, x, y);
            out_row[o + 1] = centre;
            out_row[o + 2] = left_right_checked(raw_mosaic, width, x, y);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicLinear;

impl<T> super::IDemosaic<T> for DemosaicLinear
where
    T: FornaxPrimitive,
{
    fn demosaic(&self, bayer_image: &BayerImage<T>) -> ImageBuffer<Rgb<T>, Vec<T>> {
        let mosaic: &ImageBuffer<Luma<T>, Vec<T>> = bayer_image.mosaic();
        let pattern = bayer_image.pattern();
        let (width, height) = mosaic.dimensions();
        let mut img: ImageBuffer<Rgb<T>, Vec<T>> = ImageBuffer::new(width, height);
        let bayer_mask = pattern.as_mask();

        clerk::debug!("Start demosaicing.");

        if width == 0 || height == 0 {
            clerk::debug!("End demosaicing.");
            return img;
        }

        let raw_mosaic: &[T] = mosaic.as_raw();
        let row_stride = width as usize * 3;
        let raw_out: &mut [T] = &mut img;

        raw_out
            .par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, out_row)| {
                let y = y as u32;
                let is_border_row = y == 0 || y == height - 1;

                for x in 0..width {
                    // `(x & 1) + 2 * (y & 1)` is this pixel's slot in the 2x2
                    // Bayer tile.
                    let channel =
                        unsafe { *bayer_mask.get_unchecked(((x & 1) + 2 * (y & 1)) as usize) };

                    if is_border_row || x == 0 || x == width - 1 {
                        write_pixel_checked(raw_mosaic, width, height, x, y, channel, out_row);
                    } else {
                        write_pixel_fast(raw_mosaic, width, x, y, channel, out_row);
                    }
                }
            });

        clerk::debug!("End demosaicing.");
        img
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::demosaic::IDemosaic;

    #[test]
    fn test_linear_rggb() -> mischief::Result<()> {
        let test_vec: Vec<f32> = vec![
            1.0, 2.0, 3.0, // Row 1
            4.0, 5.0, 6.0, // Row 2
            7.0, 8.0, 9.0, // Row 3
        ];

        let bayer_image = BayerImage::new(
            ImageBuffer::from_vec(3, 3, test_vec).unwrap(),
            fornax_core::BayerPattern::RGGB,
        );
        let demosaicer = DemosaicLinear;
        let output_img = demosaicer.demosaic(&bayer_image);
        let out_vec: Vec<Vec<Vec<f32>>> = output_img
            .as_raw()
            .chunks(9)
            .map(|chunk| chunk.chunks(3).map(|c| c.to_vec()).collect())
            .collect();
        println!("{:?}", out_vec[0]);
        println!("{:?}", out_vec[1]);
        println!("{:?}", out_vec[2]);
        assert_eq!(
            vec![
                vec![
                    vec![1.0, 3.0, 5.0],
                    vec![2.0, 2.0, 5.0],
                    vec![3.0, 4.0, 5.0]
                ],
                vec![
                    vec![4.0, 4.0, 5.0],
                    vec![5.0, 5.0, 5.0],
                    vec![6.0, 6.0, 5.0]
                ],
                vec![
                    vec![7.0, 6.0, 5.0],
                    vec![8.0, 8.0, 5.0],
                    vec![9.0, 7.0, 5.0]
                ]
            ],
            out_vec
        );
        Ok(())
    }
    #[test]
    fn test_linear_bggr() -> mischief::Result<()> {
        let test_vec: Vec<f32> = vec![
            1.0, 2.0, 3.0, // Row 1
            4.0, 5.0, 6.0, // Row 2
            7.0, 8.0, 9.0, // Row 3
        ];

        let bayer_image = BayerImage::new(
            ImageBuffer::from_vec(3, 3, test_vec).unwrap(),
            fornax_core::BayerPattern::BGGR,
        );
        let demosaicer = DemosaicLinear;
        let output_img = demosaicer.demosaic(&bayer_image);
        let out_vec: Vec<Vec<Vec<f32>>> = output_img
            .as_raw()
            .chunks(9)
            .map(|chunk| chunk.chunks(3).map(|c| c.to_vec()).collect())
            .collect();
        println!("{:?}", out_vec[0]);
        println!("{:?}", out_vec[1]);
        println!("{:?}", out_vec[2]);
        assert_eq!(
            vec![
                vec![
                    vec![5.0, 3.0, 1.0],
                    vec![5.0, 2.0, 2.0],
                    vec![5.0, 4.0, 3.0]
                ],
                vec![
                    vec![5.0, 4.0, 4.0],
                    vec![5.0, 5.0, 5.0],
                    vec![5.0, 6.0, 6.0]
                ],
                vec![
                    vec![5.0, 6.0, 7.0],
                    vec![5.0, 8.0, 8.0],
                    vec![5.0, 7.0, 9.0]
                ],
            ],
            out_vec
        );
        Ok(())
    }
    #[test]
    fn test_linear_grbg() -> mischief::Result<()> {
        let test_vec: Vec<f32> = vec![
            1.0, 2.0, 3.0, // Row 1
            4.0, 5.0, 6.0, // Row 2
            7.0, 8.0, 9.0, // Row 3
        ];

        let bayer_image = BayerImage::new(
            ImageBuffer::from_vec(3, 3, test_vec).unwrap(),
            fornax_core::BayerPattern::GRBG,
        );
        let demosaicer = DemosaicLinear;
        let output_img = demosaicer.demosaic(&bayer_image);
        let out_vec: Vec<Vec<Vec<f32>>> = output_img
            .as_raw()
            .chunks(9)
            .map(|chunk| chunk.chunks(3).map(|c| c.to_vec()).collect())
            .collect();
        println!("{:?}", out_vec[0]);
        println!("{:?}", out_vec[1]);
        println!("{:?}", out_vec[2]);
        assert_eq!(
            vec![
                vec![
                    vec![2.0, 1.0, 4.0],
                    vec![2.0, 3.0, 5.0],
                    vec![2.0, 3.0, 6.0]
                ],
                vec![
                    vec![5.0, 4.3333335, 4.0],
                    vec![5.0, 5.0, 5.0],
                    vec![5.0, 5.6666665, 6.0]
                ],
                vec![
                    vec![8.0, 7.0, 4.0],
                    vec![8.0, 7.0, 5.0],
                    vec![8.0, 9.0, 6.0]
                ],
            ],
            out_vec
        );
        Ok(())
    }
    #[test]
    fn test_linear_gbrg() -> mischief::Result<()> {
        let test_vec: Vec<f32> = vec![
            1.0, 2.0, 3.0, // Row 1
            4.0, 5.0, 6.0, // Row 2
            7.0, 8.0, 9.0, // Row 3
        ];

        let bayer_image = BayerImage::new(
            ImageBuffer::from_vec(3, 3, test_vec).unwrap(),
            fornax_core::BayerPattern::GBRG,
        );
        let demosaicer = DemosaicLinear;
        let output_img = demosaicer.demosaic(&bayer_image);
        let out_vec: Vec<Vec<Vec<f32>>> = output_img
            .as_raw()
            .chunks(9)
            .map(|chunk| chunk.chunks(3).map(|c| c.to_vec()).collect())
            .collect();
        println!("{:?}", out_vec[0]);
        println!("{:?}", out_vec[1]);
        println!("{:?}", out_vec[2]);
        assert_eq!(
            vec![
                vec![
                    vec![4.0, 1.0, 2.0],
                    vec![5.0, 3.0, 2.0],
                    vec![6.0, 3.0, 2.0]
                ],
                vec![
                    vec![4.0, 4.3333335, 5.0],
                    vec![5.0, 5.0, 5.0],
                    vec![6.0, 5.6666665, 5.0]
                ],
                vec![
                    vec![4.0, 7.0, 8.0],
                    vec![5.0, 7.0, 8.0],
                    vec![6.0, 9.0, 8.0]
                ],
            ],
            out_vec
        );
        Ok(())
    }
}
