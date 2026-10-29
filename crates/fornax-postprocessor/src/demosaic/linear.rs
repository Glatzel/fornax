use fornax_core::error::DemosaicError;
use fornax_core::protocol::LazyDemosaic;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive};
use generic_num::num;
const CH_R: u8 = BayerChannel::R as u8;
const CH_G: u8 = BayerChannel::GRedRow as u8;
const CH_B: u8 = BayerChannel::B as u8;

pub struct LinearLazy<'a, T> {
    raw: &'a [T],
    width: u32,
    height: u32,
    pattern: BayerPattern,
}

impl<'a, T: Primitive + Send + Sync> LazyDemosaic<T> for LinearLazy<'a, T> {
    fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let row_stride = self.width as usize * 3;
        for (dy, row) in out.chunks_exact_mut(row_stride).enumerate() {
            self.fill_row(y0 + dy, row); // the previous per-row body, unchanged
        }
    }
}
impl<'a, T: Primitive + Send + Sync> LinearLazy<'a, T> {
    fn fill_row(&self, y: usize, out: &mut [T]) {
        let (width, height) = (self.width, self.height);
        let (w, h) = (width as usize, height as usize);
        let yu = y as u32;
        let pattern = self.pattern;
        let raw = self.raw;

        if y == 0 || y == h - 1 {
            for x in 0..width {
                write_pixel_checked(raw, width, height, x, yu, pattern.channel_at(x, yu), out);
            }
            return;
        }

        let prev = &raw[(y - 1) * w..y * w];
        let cur = &raw[y * w..(y + 1) * w];
        let next = &raw[(y + 1) * w..(y + 2) * w];

        match (pattern.channel_at(0, yu), pattern.channel_at(1, yu)) {
            (BayerChannel::R, BayerChannel::GRedRow) => {
                interior_row::<T, CH_R, CH_G>(prev, cur, next, out)
            }
            (BayerChannel::GRedRow, BayerChannel::R) => {
                interior_row::<T, CH_G, CH_R>(prev, cur, next, out)
            }
            (BayerChannel::GBlueRow, BayerChannel::B) => {
                interior_row::<T, { BayerChannel::GBlueRow as u8 }, CH_B>(prev, cur, next, out)
            }
            (BayerChannel::B, BayerChannel::GBlueRow) => {
                interior_row::<T, CH_B, { BayerChannel::GBlueRow as u8 }>(prev, cur, next, out)
            }
            _ => unreachable!("rows in a Bayer tile are always R/G or G/B"),
        }

        write_pixel_checked(raw, width, height, 0, yu, pattern.channel_at(0, yu), out);
        if width > 1 {
            let x = width - 1;
            write_pixel_checked(raw, width, height, x, yu, pattern.channel_at(x, yu), out);
        }
    }
}
#[inline(always)]
const fn idx(width: u32, x: u32, y: u32) -> usize { y as usize * width as usize + x as usize }

// ---------- Checked path: used only for border rows/columns ----------

fn diagonal_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: Primitive,
{
    let mut count = 0_u32;
    let mut sum = T::zero();
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
    sum / num!(count.max(1))
}

fn neighbour_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: Primitive,
{
    let mut count = 0_u32;
    let mut sum = T::zero();
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
    sum / num!(count.max(1))
}

fn left_right_checked<T>(raw: &[T], width: u32, x: u32, y: u32) -> T
where
    T: Primitive,
{
    let mut count = 0_u32;
    let mut sum = T::zero();
    if x > 0 {
        sum = sum + raw[idx(width, x - 1, y)];
        count += 1;
    }
    if x + 1 < width {
        sum = sum + raw[idx(width, x + 1, y)];
        count += 1;
    }
    sum / num!(count.max(1))
}

fn top_down_checked<T>(raw: &[T], width: u32, height: u32, x: u32, y: u32) -> T
where
    T: Primitive,
{
    let mut count = 0_u32;
    let mut sum = T::zero();
    if y > 0 {
        sum = sum + raw[idx(width, x, y - 1)];
        count += 1;
    }
    if y + 1 < height {
        sum = sum + raw[idx(width, x, y + 1)];
        count += 1;
    }
    sum / num!(count.max(1))
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
    T: Primitive,
{
    let o = x as usize * 3;
    let centre = raw_mosaic[idx(width, x, y)];
    match channel {
        BayerChannel::R => {
            out_row[o] = centre;
            out_row[o + 1] = neighbour_checked(raw_mosaic, width, height, x, y);
            out_row[o + 2] = diagonal_checked(raw_mosaic, width, height, x, y);
        }
        BayerChannel::GRedRow => {
            out_row[o] = left_right_checked(raw_mosaic, width, x, y);
            out_row[o + 1] = centre;
            out_row[o + 2] = top_down_checked(raw_mosaic, width, height, x, y);
        }
        BayerChannel::B => {
            out_row[o] = diagonal_checked(raw_mosaic, width, height, x, y);
            out_row[o + 1] = neighbour_checked(raw_mosaic, width, height, x, y);
            out_row[o + 2] = centre;
        }
        BayerChannel::GBlueRow => {
            out_row[o] = top_down_checked(raw_mosaic, width, height, x, y);
            out_row[o + 1] = centre;
            out_row[o + 2] = left_right_checked(raw_mosaic, width, x, y);
        }
    }
}

// ---------- Fast path: interior pixels, channel known at compile time
// ----------

/// Interpolates one interior pixel (`1 <= x < w-1`) of channel `CH`.
/// `p`, `c`, `n` are the previous, current and next mosaic rows.
#[inline(always)]
fn interior_px<T, const CH: u8>(p: &[T], c: &[T], n: &[T], x: usize, out: &mut [T])
where
    T: Primitive,
{
    let two = num!(2);
    let four = num!(4);
    let (l, r, t, b) = (c[x - 1], c[x + 1], p[x], n[x]);
    let ctr = c[x];
    let rgb = match CH {
        CH_R | CH_B => {
            let cross = (l + r + t + b) / four;
            let diag = (p[x - 1] + p[x + 1] + n[x - 1] + n[x + 1]) / four;
            if CH == CH_R {
                [ctr, cross, diag]
            } else {
                [diag, cross, ctr]
            }
        }
        CH_G => [(l + r) / two, ctr, (t + b) / two],
        // G2
        _ => [(t + b) / two, ctr, (l + r) / two],
    };
    out[x * 3..x * 3 + 3].copy_from_slice(&rgb);
}

/// Fills `x` in `1..w-1` of one row. `CE` / `CO` are the channels at even /
/// odd `x`.
#[inline(always)]
fn interior_row<T, const CE: u8, const CO: u8>(prev: &[T], cur: &[T], next: &[T], out: &mut [T])
where
    T: Primitive,
{
    let w = cur.len();
    assert!(prev.len() == w && next.len() == w && out.len() == 3 * w);
    let mut x = 1; // odd -> CO, then even -> CE, ...
    while x + 2 < w {
        interior_px::<T, CO>(prev, cur, next, x, out);
        interior_px::<T, CE>(prev, cur, next, x + 1, out);
        x += 2;
    }
    if x + 1 < w {
        interior_px::<T, CO>(prev, cur, next, x, out);
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicLinear;

impl DemosaicLinear {
    pub fn lazy<'a, T: Primitive + Send + Sync>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<LinearLazy<'a, T>, DemosaicError> {
        Ok(LinearLazy {
            raw: bayer.as_raw(),
            width: bayer.width(),
            height: bayer.height(),
            pattern,
        })
    }
}
#[cfg(test)]
mod tests {

    use image::ImageBuffer;

    use super::*;

    #[test]
    fn test_linear_rggb() -> mischief::Result<()> {
        let test_vec: Vec<f32> = vec![
            1.0, 2.0, 3.0, // Row 1
            4.0, 5.0, 6.0, // Row 2
            7.0, 8.0, 9.0, // Row 3
        ];

        let bayer_image = ImageBuffer::from_vec(3, 3, test_vec).unwrap();
        let output_img = DemosaicLinear
            .lazy(&bayer_image, BayerPattern::RGGB)?
            .collect();
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

        let bayer_image = ImageBuffer::from_vec(3, 3, test_vec).unwrap();
        let output_img = DemosaicLinear
            .lazy(&bayer_image, BayerPattern::BGGR)?
            .collect();
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

        let bayer_image = ImageBuffer::from_vec(3, 3, test_vec).unwrap();
        let output_img = DemosaicLinear
            .lazy(&bayer_image, BayerPattern::GRBG)?
            .collect();
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

        let bayer_image = ImageBuffer::from_vec(3, 3, test_vec).unwrap();
        let output_img = DemosaicLinear
            .lazy(&bayer_image, BayerPattern::GBRG)?
            .collect();
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
