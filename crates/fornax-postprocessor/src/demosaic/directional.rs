use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, channel, dense_from_f32, normalized};

#[derive(Debug, Clone, Copy)]
pub(super) enum Profile {
    Dcb,
    Dht,
    Aahd,
    Amaze,
    Lmmse,
}

pub(super) fn lazy<T>(
    image: &MonoImage<T>,
    pattern: BayerPattern,
    profile: Profile,
) -> Result<DenseLazy<T>, fornax_core::error::DemosaicError>
where
    T: Primitive + Send + Sync,
{
    if image.width() < 5 || image.height() < 5 {
        return Err(fornax_core::error::DemosaicError::InvalidInput(
            "directional demosaic requires an image at least 5x5".into(),
        ));
    }
    let output = run(
        &normalized(image),
        image.width() as usize,
        image.height() as usize,
        pattern,
        profile,
    );
    Ok(DenseLazy::new(dense_from_f32(
        image.width(),
        image.height(),
        output,
    )))
}

fn run(
    input: &[f32],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    profile: Profile,
) -> Vec<f32> {
    let mut green = vec![0.0_f32; width * height];
    let mut rgb = vec![[0.0_f32; 3]; width * height];
    let (green_power, chroma_radius, refinement) = match profile {
        Profile::Dcb => (2.0, 2, 0),
        Profile::Dht => (1.0, 2, 0),
        Profile::Aahd => (4.0, 3, 0),
        Profile::Amaze => (3.0, 3, 0),
        Profile::Lmmse => (1.0, 2, 1),
    };

    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let measured = channel(pattern, x as u32, y as u32);
            rgb[index][measured] = input[index];
            if measured == 1 {
                green[index] = input[index];
            }
        }
    }

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y * width + x;
            if green[index] != 0.0 || channel(pattern, x as u32, y as u32) == 1 {
                continue;
            }
            let center = input[index];
            let west = at(input, width, x - 1, y);
            let east = at(input, width, x + 1, y);
            let north = at(input, width, x, y - 1);
            let south = at(input, width, x, y + 1);
            let west2 = at(input, width, x.saturating_sub(2), y);
            let east2 = at(input, width, (x + 2).min(width - 1), y);
            let north2 = at(input, width, x, y.saturating_sub(2));
            let south2 = at(input, width, x, (y + 2).min(height - 1));

            let horizontal = (2.0_f32.mul_add(center, -west2) - east2).mul_add(0.25, f32::midpoint(west, east))
                .clamp(west.min(east), west.max(east));
            let vertical = (2.0_f32.mul_add(center, -north2) - south2).mul_add(0.25, f32::midpoint(north, south))
                .clamp(north.min(south), north.max(south));
            let horizontal_gradient = (west - east).abs()
                + (center - west2).abs()
                + (center - east2).abs()
                + (west - center).abs()
                + (east - center).abs();
            let vertical_gradient = (north - south).abs()
                + (center - north2).abs()
                + (center - south2).abs()
                + (north - center).abs()
                + (south - center).abs();
            let horizontal_weight = 1.0 / (1.0e-5 + horizontal_gradient).powf(green_power);
            let vertical_weight = 1.0 / (1.0e-5 + vertical_gradient).powf(green_power);
            green[index] = match profile {
                Profile::Dht => {
                    let horizontal_ratio =
                        center * f32::midpoint(west, east) / (f32::midpoint(west2, east2) + 1.0e-5);
                    let vertical_ratio =
                        center * f32::midpoint(north, south) / (f32::midpoint(north2, south2) + 1.0e-5);
                    vertical_ratio.mul_add(vertical_weight, horizontal_ratio * horizontal_weight)
                        / (horizontal_weight + vertical_weight)
                }
                Profile::Lmmse => {
                    vertical.mul_add(vertical_weight, horizontal * horizontal_weight)
                        / (horizontal_weight + vertical_weight)
                }
                _ => {
                    if horizontal_gradient < vertical_gradient {
                        horizontal
                    } else if vertical_gradient < horizontal_gradient {
                        vertical
                    } else {
                        f32::midpoint(horizontal, vertical)
                    }
                }
            }
            .clamp(0.0, 1.0);
        }
    }
    fill_green_border(&mut green, input, width, height, pattern);

    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let measured = channel(pattern, x as u32, y as u32);
            rgb[index][measured] = input[index];
            rgb[index][1] = green[index];
            for target in [0, 2] {
                if target == measured {
                    continue;
                }
                let mut sum = 0.0;
                let mut weight_sum = 0.0;
                for dy in -chroma_radius..=chroma_radius {
                    for dx in -chroma_radius..=chroma_radius {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if channel(pattern, nx as u32, ny as u32) != target {
                            continue;
                        }
                        let neighbor = ny * width + nx;
                        let distance = (dx * dx + dy * dy) as f32;
                        let color_edge = (input[neighbor] - green[neighbor]).abs();
                        let weight = 1.0 / (0.25 + distance + color_edge);
                        sum = (green[index] + input[neighbor] - green[neighbor]).mul_add(weight, sum);
                        weight_sum += weight;
                    }
                }
                if weight_sum > 0.0 {
                    rgb[index][target] = sum / weight_sum;
                }
            }
        }
    }

    for _ in 0..refinement {
        let previous = rgb.clone();
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let index = y * width + x;
                let measured = channel(pattern, x as u32, y as u32);
                for target in [0, 1, 2] {
                    if target == measured {
                        continue;
                    }
                    let mut differences = [0.0_f32; 4];
                    let neighbors = [index - 1, index + 1, index - width, index + width];
                    for (slot, neighbor) in neighbors.into_iter().enumerate() {
                        differences[slot] = previous[neighbor][target] - green[neighbor];
                    }
                    differences.sort_by(f32::total_cmp);
                    let color_difference = f32::midpoint(differences[1], differences[2]);
                    let refined = green[index] + color_difference;
                    rgb[index][target] = match profile {
                        Profile::Lmmse => refined.mul_add(0.5, previous[index][target] * 0.5),
                        Profile::Amaze => refined.mul_add(0.25, previous[index][target] * 0.75),
                        Profile::Dcb => refined.mul_add(0.4, previous[index][target] * 0.6),
                        Profile::Dht | Profile::Aahd => {
                            refined.mul_add(0.2, previous[index][target] * 0.8)
                        }
                    }
                    .clamp(0.0, 1.0);
                }
            }
        }
    }

    rgb.into_iter().flatten().collect()
}

fn fill_green_border(
    green: &mut [f32],
    input: &[f32],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) {
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if green[index] != 0.0 || channel(pattern, x as u32, y as u32) == 1 {
                if channel(pattern, x as u32, y as u32) == 1 {
                    green[index] = input[index];
                }
                continue;
            }
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in -1_i32..=1 {
                for dx in -1_i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let neighbor = ny as usize * width + nx as usize;
                    if channel(pattern, nx as u32, ny as u32) == 1 {
                        sum += input[neighbor];
                        count += 1.0;
                    }
                }
            }
            green[index] = if count > 0.0 {
                sum / count
            } else {
                input[index]
            };
        }
    }
}

const fn at(values: &[f32], width: usize, x: usize, y: usize) -> f32 { values[y * width + x] }
