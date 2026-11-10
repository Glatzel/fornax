use fornax_raw_core::error::DecodeError;

use super::corrupt;

const MAX_OPCODES: usize = 4096;
const MAX_ELEMENTS: usize = 1 << 24;
const BAYER_PERIOD: usize = 2;

#[derive(Debug, Clone, Copy)]
struct Region {
    top: u32,
    left: u32,
    bottom: u32,
    right: u32,
    plane: u32,
    planes: u32,
    row_pitch: u32,
    col_pitch: u32,
}

#[derive(Debug)]
enum Opcode {
    Unsupported(&'static str),
    FixVignetteRadial {
        coefficients: [f64; 5],
        center_x: f64,
        center_y: f64,
    },
    FixBadPixelsConstant {
        constant: u32,
    },
    FixBadPixelsList {
        points: Vec<(u32, u32)>,
        rectangles: Vec<(u32, u32, u32, u32)>,
    },
    MapTable {
        region: Region,
        table: Vec<u16>,
    },
    MapPolynomial {
        region: Region,
        coefficients: Vec<f64>,
    },
    GainMap {
        region: Region,
        points_v: u32,
        points_h: u32,
        spacing_v: f64,
        spacing_h: f64,
        origin_v: f64,
        origin_h: f64,
        map_planes: u32,
        gains: Vec<f32>,
    },
    DeltaPerRow {
        region: Region,
        values: Vec<f32>,
    },
    DeltaPerColumn {
        region: Region,
        values: Vec<f32>,
    },
    ScalePerRow {
        region: Region,
        values: Vec<f32>,
    },
    ScalePerColumn {
        region: Region,
        values: Vec<f32>,
    },
}

impl Opcode {
    fn unsupported_name(&self) -> Option<&'static str> {
        match self {
            Self::Unsupported(name) => Some(name),
            _ => None,
        }
    }
}

pub(super) fn apply(
    bytes: &[u8],
    samples: &mut [f32],
    width: usize,
    height: usize,
) -> Result<(), DecodeError> {
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| corrupt("opcode image sample count overflows"))?;
    if width == 0
        || height == 0
        || samples.len() != expected
        || samples.iter().any(|sample| !sample.is_finite())
    {
        return Err(corrupt("opcode image geometry or samples are invalid"));
    }

    let opcodes = parse(bytes)?;
    let unsupported = opcodes
        .iter()
        .filter_map(Opcode::unsupported_name)
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        return Err(corrupt(format!(
            "OpcodeList1 contains operations outside the Bayer raw domain: {}",
            unsupported.join(", ")
        )));
    }

    for opcode in opcodes {
        apply_opcode(&opcode, samples, width, height);
    }
    Ok(())
}

fn parse(bytes: &[u8]) -> Result<Vec<Opcode>, DecodeError> {
    let mut cursor = Cursor::new(bytes);
    let count = cursor.u32()? as usize;
    if count > MAX_OPCODES {
        return Err(corrupt(format!(
            "invalid DNG opcode list with {count} opcodes"
        )));
    }
    let mut opcodes = Vec::with_capacity(count);
    for _ in 0..count {
        let id = cursor.u32()?;
        let _version = cursor.u32()?;
        let _flags = cursor.u32()?;
        let length = cursor.u32()? as usize;
        let body = cursor.take(length)?;
        let mut body = Cursor::new(body);
        opcodes.push(match id {
            1 => Opcode::Unsupported("WarpRectilinear"),
            2 => Opcode::Unsupported("WarpFisheye"),
            3 => {
                let mut values = [0.0; 7];
                for value in &mut values {
                    *value = body.f64()?;
                }
                Opcode::FixVignetteRadial {
                    coefficients: [values[0], values[1], values[2], values[3], values[4]],
                    center_x: values[5],
                    center_y: values[6],
                }
            }
            4 => Opcode::FixBadPixelsConstant {
                constant: body.u32()?,
            },
            5 => {
                let _bayer_phase = body.u32()?;
                let point_count = body.u32()? as usize;
                let rectangle_count = body.u32()? as usize;
                let required = point_count
                    .checked_mul(8)
                    .and_then(|bytes| {
                        rectangle_count
                            .checked_mul(16)
                            .and_then(|rect_bytes| bytes.checked_add(rect_bytes))
                    })
                    .ok_or_else(|| corrupt("FixBadPixelsList element count overflows"))?;
                if required > body.remaining() {
                    return Err(corrupt("FixBadPixelsList counts exceed opcode size"));
                }
                let points = (0..point_count)
                    .map(|_| Ok((body.u32()?, body.u32()?)))
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                let rectangles = (0..rectangle_count)
                    .map(|_| Ok((body.u32()?, body.u32()?, body.u32()?, body.u32()?)))
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                Opcode::FixBadPixelsList { points, rectangles }
            }
            6 => Opcode::Unsupported("TrimBounds"),
            7 => {
                let region = body.region()?;
                let count = body.count(2)?;
                let table = (0..count)
                    .map(|_| body.u16())
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                Opcode::MapTable { region, table }
            }
            8 => {
                let region = body.region()?;
                let degree = body.u32()? as usize;
                if degree > 8 || (degree + 1) * 8 > body.remaining() {
                    return Err(corrupt("invalid MapPolynomial degree"));
                }
                let coefficients = (0..=degree)
                    .map(|_| body.f64())
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                Opcode::MapPolynomial {
                    region,
                    coefficients,
                }
            }
            9 => {
                let region = body.region()?;
                let points_v = body.u32()?;
                let points_h = body.u32()?;
                let spacing_v = body.f64()?;
                let spacing_h = body.f64()?;
                let origin_v = body.f64()?;
                let origin_h = body.f64()?;
                let map_planes = body.u32()?.max(1);
                let total = (points_v as usize)
                    .checked_mul(points_h as usize)
                    .and_then(|points| points.checked_mul(map_planes as usize))
                    .ok_or_else(|| corrupt("invalid GainMap dimensions"))?;
                if total == 0 || total > MAX_ELEMENTS || total.saturating_mul(4) > body.remaining()
                {
                    return Err(corrupt("invalid GainMap dimensions"));
                }
                let gains = (0..total)
                    .map(|_| body.f32())
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                Opcode::GainMap {
                    region,
                    points_v,
                    points_h,
                    spacing_v,
                    spacing_h,
                    origin_v,
                    origin_h,
                    map_planes,
                    gains,
                }
            }
            10..=13 => {
                let region = body.region()?;
                let count = body.count(4)?;
                let values = (0..count)
                    .map(|_| body.f32())
                    .collect::<Result<Vec<_>, DecodeError>>()?;
                match id {
                    10 => Opcode::DeltaPerRow { region, values },
                    11 => Opcode::DeltaPerColumn { region, values },
                    12 => Opcode::ScalePerRow { region, values },
                    _ => Opcode::ScalePerColumn { region, values },
                }
            }
            _ => Opcode::Unsupported("Other"),
        });
    }
    Ok(opcodes)
}

fn apply_opcode(opcode: &Opcode, samples: &mut [f32], width: usize, height: usize) {
    match opcode {
        Opcode::Unsupported(_) => {
            unreachable!("unsupported opcodes are rejected before application")
        }
        Opcode::FixVignetteRadial {
            coefficients,
            center_x,
            center_y,
        } => apply_vignette(samples, width, height, coefficients, *center_x, *center_y),
        Opcode::FixBadPixelsConstant { constant } => {
            let constant = *constant as f32;
            let bad = samples
                .iter()
                .enumerate()
                .filter_map(|(index, sample)| (*sample == constant).then_some(index))
                .collect();
            repair_bad_pixels(samples, width, height, bad);
        }
        Opcode::FixBadPixelsList { points, rectangles } => {
            let mut bad = Vec::new();
            for &(row, column) in points {
                if let Some(index) = checked_index(row, column, width, height) {
                    bad.push(index);
                }
            }
            for &(top, left, bottom, right) in rectangles {
                for row in top..bottom.min(height as u32) {
                    for column in left..right.min(width as u32) {
                        bad.push(row as usize * width + column as usize);
                    }
                }
            }
            repair_bad_pixels(samples, width, height, bad);
        }
        Opcode::MapTable { region, table } if !table.is_empty() => {
            let last = (table.len() - 1) as f32;
            for_each_region(*region, width, height, |index, _, _| {
                let table_index = samples[index].round().clamp(0.0, last) as usize;
                samples[index] = table[table_index] as f32;
            });
        }
        Opcode::MapTable { .. } => {}
        Opcode::MapPolynomial {
            region,
            coefficients,
        } => {
            for_each_region(*region, width, height, |index, _, _| {
                let x = samples[index] as f64 / 65_535.0;
                let mut value = 0.0;
                for coefficient in coefficients.iter().rev() {
                    value = value * x + coefficient;
                }
                samples[index] = (value * 65_535.0) as f32;
            });
        }
        Opcode::GainMap {
            region,
            points_v,
            points_h,
            spacing_v,
            spacing_h,
            origin_v,
            origin_h,
            map_planes,
            gains,
        } => apply_gain_map(
            samples,
            width,
            height,
            *region,
            *points_v,
            *points_h,
            *spacing_v,
            *spacing_h,
            *origin_v,
            *origin_h,
            *map_planes,
            gains,
        ),
        Opcode::DeltaPerRow { region, values } => {
            for_each_region(*region, width, height, |index, _, row| {
                let offset = (row - region.top as usize) / region.row_pitch as usize;
                samples[index] += values.get(offset).copied().unwrap_or(0.0) * 65_535.0;
            });
        }
        Opcode::DeltaPerColumn { region, values } => {
            for_each_region(*region, width, height, |index, column, _| {
                let offset = (column - region.left as usize) / region.col_pitch as usize;
                samples[index] += values.get(offset).copied().unwrap_or(0.0) * 65_535.0;
            });
        }
        Opcode::ScalePerRow { region, values } => {
            for_each_region(*region, width, height, |index, _, row| {
                let offset = (row - region.top as usize) / region.row_pitch as usize;
                samples[index] *= values.get(offset).copied().unwrap_or(1.0);
            });
        }
        Opcode::ScalePerColumn { region, values } => {
            for_each_region(*region, width, height, |index, column, _| {
                let offset = (column - region.left as usize) / region.col_pitch as usize;
                samples[index] *= values.get(offset).copied().unwrap_or(1.0);
            });
        }
    }
}

fn for_each_region(
    region: Region,
    width: usize,
    height: usize,
    mut operation: impl FnMut(usize, usize, usize),
) {
    if region.plane != 0 || region.planes == 0 {
        return;
    }
    let top = region.top as usize;
    let bottom = (region.bottom as usize).min(height);
    let left = region.left as usize;
    let right = (region.right as usize).min(width);
    for row in (top..bottom).step_by(region.row_pitch as usize) {
        for column in (left..right).step_by(region.col_pitch as usize) {
            operation(row * width + column, column, row);
        }
    }
}

fn apply_vignette(
    samples: &mut [f32],
    width: usize,
    height: usize,
    coefficients: &[f64; 5],
    center_x: f64,
    center_y: f64,
) {
    let center_x = center_x * width as f64;
    let center_y = center_y * height as f64;
    let max_distance_squared = [
        (0.0, 0.0),
        (width as f64, 0.0),
        (0.0, height as f64),
        (width as f64, height as f64),
    ]
    .into_iter()
    .map(|(x, y)| (x - center_x).powi(2) + (y - center_y).powi(2))
    .fold(0.0_f64, f64::max);
    if max_distance_squared <= 0.0 || !max_distance_squared.is_finite() {
        return;
    }
    for row in 0..height {
        let row_distance = (row as f64 - center_y).powi(2);
        for column in 0..width {
            let radius_squared =
                ((column as f64 - center_x).powi(2) + row_distance) / max_distance_squared;
            let gain = 1.0
                + radius_squared
                    * (coefficients[0]
                        + radius_squared
                            * (coefficients[1]
                                + radius_squared
                                    * (coefficients[2]
                                        + radius_squared
                                            * (coefficients[3]
                                                + radius_squared * coefficients[4]))));
            samples[row * width + column] *= gain as f32;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_gain_map(
    samples: &mut [f32],
    width: usize,
    height: usize,
    region: Region,
    points_v: u32,
    points_h: u32,
    spacing_v: f64,
    spacing_h: f64,
    origin_v: f64,
    origin_h: f64,
    map_planes: u32,
    gains: &[f32],
) {
    let points_v = points_v as usize;
    let points_h = points_h as usize;
    let map_planes = map_planes as usize;
    if points_v == 0
        || points_h == 0
        || map_planes == 0
        || gains.len() < points_v * points_h * map_planes
        || region.plane != 0
        || region.planes == 0
    {
        return;
    }
    let top = region.top as usize;
    let bottom = (region.bottom as usize).min(height);
    let left = region.left as usize;
    let right = (region.right as usize).min(width);
    for row in (top..bottom).step_by(region.row_pitch as usize) {
        let (v0, v1, vertical_weight) =
            interpolate_index(row as f64 / height as f64, origin_v, spacing_v, points_v);
        for column in (left..right).step_by(region.col_pitch as usize) {
            let (h0, h1, horizontal_weight) =
                interpolate_index(column as f64 / width as f64, origin_h, spacing_h, points_h);
            let top_left = gains[(v0 * points_h + h0) * map_planes];
            let top_right = gains[(v0 * points_h + h1) * map_planes];
            let bottom_left = gains[(v1 * points_h + h0) * map_planes];
            let bottom_right = gains[(v1 * points_h + h1) * map_planes];
            let top_gain = top_left + (top_right - top_left) * horizontal_weight;
            let bottom_gain = bottom_left + (bottom_right - bottom_left) * horizontal_weight;
            samples[row * width + column] *= top_gain + (bottom_gain - top_gain) * vertical_weight;
        }
    }
}

fn interpolate_index(
    position: f64,
    origin: f64,
    spacing: f64,
    count: usize,
) -> (usize, usize, f32) {
    let coordinate = if spacing > 0.0 {
        ((position - origin) / spacing).clamp(0.0, (count - 1) as f64)
    } else {
        0.0
    };
    let lower = (coordinate.floor() as usize).min(count - 1);
    (
        lower,
        (lower + 1).min(count - 1),
        (coordinate - lower as f64) as f32,
    )
}

fn checked_index(row: u32, column: u32, width: usize, height: usize) -> Option<usize> {
    ((row as usize) < height && (column as usize) < width)
        .then_some(row as usize * width + column as usize)
}

fn repair_bad_pixels(samples: &mut [f32], width: usize, height: usize, mut bad: Vec<usize>) {
    bad.sort_unstable();
    bad.dedup();
    let mut is_bad = vec![false; samples.len()];
    for &index in &bad {
        is_bad[index] = true;
    }
    let mut fixes = Vec::with_capacity(bad.len());
    for index in bad {
        let row = index / width;
        let column = index % width;
        let mut sum = 0.0;
        let mut count = 0.0;
        for (dx, dy) in [
            (-1_isize, 0_isize),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
        ] {
            let neighbor_x = column as isize + dx * BAYER_PERIOD as isize;
            let neighbor_y = row as isize + dy * BAYER_PERIOD as isize;
            if neighbor_x < 0
                || neighbor_y < 0
                || neighbor_x as usize >= width
                || neighbor_y as usize >= height
            {
                continue;
            }
            let neighbor = neighbor_y as usize * width + neighbor_x as usize;
            if !is_bad[neighbor] {
                sum += samples[neighbor];
                count += 1.0;
            }
        }
        if count > 0.0 {
            fixes.push((index, sum / count));
        }
    }
    for (index, value) in fixes {
        samples[index] = value;
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    const fn remaining(&self) -> usize { self.bytes.len().saturating_sub(self.position) }

    fn take(&mut self, count: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| corrupt("DNG opcode byte range overflows"))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| corrupt("DNG opcode body is truncated"))?;
        self.position = end;
        Ok(bytes)
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().expect("two bytes"),
        ))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn f32(&mut self) -> Result<f32, DecodeError> { Ok(f32::from_bits(self.u32()?)) }

    fn f64(&mut self) -> Result<f64, DecodeError> {
        let bytes = self.take(8)?.try_into().expect("eight bytes");
        Ok(f64::from_bits(u64::from_be_bytes(bytes)))
    }

    fn count(&mut self, element_bytes: usize) -> Result<usize, DecodeError> {
        let count = self.u32()? as usize;
        if count > MAX_ELEMENTS || count.saturating_mul(element_bytes) > self.remaining() {
            return Err(corrupt(format!("invalid DNG opcode with {count} elements")));
        }
        Ok(count)
    }

    fn region(&mut self) -> Result<Region, DecodeError> {
        Ok(Region {
            top: self.u32()?,
            left: self.u32()?,
            bottom: self.u32()?,
            right: self.u32()?,
            plane: self.u32()?,
            planes: self.u32()?,
            row_pitch: self.u32()?.max(1),
            col_pitch: self.u32()?.max(1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::apply;

    fn opcode(id: u32, body: &[u8]) -> Vec<u8> {
        let mut bytes = id.to_be_bytes().to_vec();
        bytes.extend_from_slice(&0x0103_0000_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&u32::try_from(body.len()).unwrap().to_be_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    fn list(opcodes: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = u32::try_from(opcodes.len()).unwrap().to_be_bytes().to_vec();
        for opcode in opcodes {
            bytes.extend_from_slice(opcode);
        }
        bytes
    }

    fn region(top: u32, left: u32, bottom: u32, right: u32, pitch: u32) -> Vec<u8> {
        [top, left, bottom, right, 0, 1, pitch, pitch]
            .into_iter()
            .flat_map(|value| value.to_be_bytes())
            .collect()
    }

    fn all_region() -> Vec<u8> { region(0, 0, u32::MAX, u32::MAX, 1) }

    #[test]
    fn map_table_rounds_and_clamps_indices() {
        let mut body = all_region();
        body.extend_from_slice(&3_u32.to_be_bytes());
        for value in [10_u16, 20, 30] {
            body.extend_from_slice(&value.to_be_bytes());
        }
        let mut samples = [0.49, 1.49, 2.5, 100.0];

        apply(&list(&[opcode(7, &body)]), &mut samples, 4, 1).unwrap();

        assert_eq!(samples, [10.0, 20.0, 30.0, 30.0]);
    }

    #[test]
    fn map_polynomial_uses_the_sixteen_bit_sample_domain() {
        let mut body = all_region();
        body.extend_from_slice(&1_u32.to_be_bytes());
        for coefficient in [0.25_f64, 0.5] {
            body.extend_from_slice(&coefficient.to_bits().to_be_bytes());
        }
        let mut samples = [0.0, 32_768.0];

        apply(&list(&[opcode(8, &body)]), &mut samples, 2, 1).unwrap();

        assert!((samples[0] - 16_383.75).abs() < 0.001, "{samples:?}");
        assert!((samples[1] - 32_767.75).abs() < 0.001, "{samples:?}");
    }

    #[test]
    fn gain_map_interpolates_before_scale() {
        let mut gain = region(0, 0, 2, 2, 1);
        gain.extend_from_slice(&2_u32.to_be_bytes());
        gain.extend_from_slice(&2_u32.to_be_bytes());
        for value in [1.0_f64, 1.0, 0.0, 0.0] {
            gain.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        gain.extend_from_slice(&1_u32.to_be_bytes());
        for value in [1.0_f32, 2.0, 3.0, 4.0] {
            gain.extend_from_slice(&value.to_bits().to_be_bytes());
        }

        let mut scale = region(0, 0, 2, 2, 1);
        scale.extend_from_slice(&2_u32.to_be_bytes());
        scale.extend_from_slice(&1.0_f32.to_bits().to_be_bytes());
        scale.extend_from_slice(&0.5_f32.to_bits().to_be_bytes());

        let mut samples = [1.0; 4];
        apply(
            &list(&[opcode(9, &gain), opcode(12, &scale)]),
            &mut samples,
            2,
            2,
        )
        .unwrap();

        assert_eq!(samples, [1.0, 1.5, 1.0, 1.25]);
    }

    #[test]
    fn row_and_column_deltas_use_the_region_origin() {
        let mut rows = region(0, 0, 2, 2, 1);
        rows.extend_from_slice(&2_u32.to_be_bytes());
        for value in [0.0001_f32, 0.0002] {
            rows.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        let mut columns = region(0, 0, 2, 2, 1);
        columns.extend_from_slice(&2_u32.to_be_bytes());
        for value in [0.0003_f32, 0.0004] {
            columns.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        let mut samples = [0.0; 4];

        apply(
            &list(&[opcode(10, &rows), opcode(11, &columns)]),
            &mut samples,
            2,
            2,
        )
        .unwrap();

        for (actual, expected) in samples
            .into_iter()
            .zip([26.214, 32.767_5, 32.767_5, 39.321])
        {
            assert!((actual - expected).abs() < 0.02, "{actual} != {expected}");
        }
    }

    #[test]
    fn vignette_normalizes_to_the_farthest_corner() {
        let mut body = Vec::new();
        for value in [0.5_f64, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5] {
            body.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        let mut samples = [1.0; 16];

        apply(&list(&[opcode(3, &body)]), &mut samples, 4, 4).unwrap();

        assert!((samples[0] - 1.5).abs() < 1e-6);
        assert!((samples[2 * 4 + 2] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn bad_pixel_repairs_use_nearest_same_colour_neighbours() {
        let mut body = 0_u32.to_be_bytes().to_vec();
        body.extend_from_slice(&1_u32.to_be_bytes());
        body.extend_from_slice(&0_u32.to_be_bytes());
        body.extend_from_slice(&2_u32.to_be_bytes());
        body.extend_from_slice(&2_u32.to_be_bytes());
        let mut samples = [10.0; 36];
        samples[2 * 6 + 2] = 1000.0;

        apply(&list(&[opcode(5, &body)]), &mut samples, 6, 6).unwrap();

        assert_eq!(samples[2 * 6 + 2], 10.0);
    }

    #[test]
    fn unsupported_operations_are_reported() {
        let error = apply(&list(&[opcode(1, &[])]), &mut [1.0], 1, 1).unwrap_err();
        assert!(error.to_string().contains("WarpRectilinear"));

        let error = apply(&list(&[opcode(99, &[])]), &mut [1.0], 1, 1).unwrap_err();
        assert!(error.to_string().contains("Other"));
    }

    #[test]
    fn malformed_lists_are_rejected() {
        assert!(apply(&[0, 0, 0, 1], &mut [1.0], 1, 1).is_err());

        let mut oversized = 4097_u32.to_be_bytes().to_vec();
        oversized.extend_from_slice(&opcode(4, &[0; 8]));
        assert!(apply(&oversized, &mut [1.0], 1, 1).is_err());

        let mut body = all_region();
        body.extend_from_slice(&u32::MAX.to_be_bytes());
        assert!(apply(&list(&[opcode(10, &body)]), &mut [1.0], 1, 1).is_err());
    }
}
