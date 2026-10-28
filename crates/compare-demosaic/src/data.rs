use fornax_devtool::synthetic_mono_image;

pub const CANONICAL_WIDTH: u32 = 512;
pub const CANONICAL_HEIGHT: u32 = 512;

#[derive(Clone, Debug)]
pub struct Input {
    #[allow(dead_code, reason = "used in comparison test diagnostics")]
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
}

pub fn canonical() -> Input { seeded("canonical", CANONICAL_WIDTH, CANONICAL_HEIGHT) }

#[cfg(test)]
pub fn test_inputs() -> Vec<Input> {
    vec![
        seeded("canonical", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        seeded("odd", 513, 511),
        seeded("partial-row-chunk", 34, 513),
        seeded("tiny", 3, 3),
        saturated("saturated", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        saturated("saturated-tiny", 3, 3),
    ]
}

fn seeded(label: &str, width: u32, height: u32) -> Input {
    let image = synthetic_mono_image::<f32>((width, height), 65_535.0);
    Input {
        label: label.to_owned(),
        width,
        height,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
            .collect(),
    }
}

#[cfg(test)]
fn saturated(label: &str, width: u32, height: u32) -> Input {
    Input {
        label: label.to_owned(),
        width,
        height,
        codes: vec![u16::MAX; width as usize * height as usize],
    }
}
