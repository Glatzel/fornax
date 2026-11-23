use fornax_codec::CodecError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanasonicV8Strip {
    pub source_offset: usize,
    pub data_bits: u32,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl PanasonicV8Strip {
    pub fn new(
        source_offset: usize,
        data_bits: u32,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<Self, CodecError> {
        if data_bits == 0 {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 strip bit count must not be zero".into(),
            ));
        }
        if width == 0 || height == 0 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 strip dimensions must be positive and even".into(),
            ));
        }
        x.checked_add(width)
            .and_then(|_| y.checked_add(height))
            .ok_or_else(|| {
                CodecError::InvalidInput("Panasonic V8 strip placement overflows usize".into())
            })?;
        Ok(Self {
            source_offset,
            data_bits,
            x,
            y,
            width,
            height,
        })
    }

    pub const fn source_offset(self) -> usize { self.source_offset }

    pub const fn data_bits(self) -> u32 { self.data_bits }

    pub const fn x(self) -> usize { self.x }

    pub const fn y(self) -> usize { self.y }

    pub const fn width(self) -> usize { self.width }

    pub const fn height(self) -> usize { self.height }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanasonicV8Params {
    pub gamma_points: [u32; 6],
    pub gamma_slopes: [u32; 6],
    pub clip_value: u16,
    pub initial_predictors: [u16; 4],
    pub huffman_table: Vec<(u8, u16)>,
    pub shift_down: Vec<u16>,
    pub strips: Vec<PanasonicV8Strip>,
}

impl PanasonicV8Params {
    pub fn new(
        gamma_points: Vec<u32>,
        gamma_slopes: Vec<u32>,
        clip_value: u16,
        initial_predictors: [u16; 4],
        huffman_table: Vec<(u8, u16)>,
        shift_down: Vec<u16>,
        strips: Vec<PanasonicV8Strip>,
    ) -> Result<Self, CodecError> {
        let gamma_points: [u32; 6] = gamma_points.try_into().map_err(|values: Vec<u32>| {
            CodecError::InvalidInput(
                format!(
                    "Panasonic V8 requires 6 gamma points, found {}",
                    values.len()
                )
                .into(),
            )
        })?;
        let gamma_slopes: [u32; 6] = gamma_slopes.try_into().map_err(|values: Vec<u32>| {
            CodecError::InvalidInput(
                format!(
                    "Panasonic V8 requires 6 gamma slopes, found {}",
                    values.len()
                )
                .into(),
            )
        })?;
        if clip_value == 0 {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 clip value must not be zero".into(),
            ));
        }
        if huffman_table.is_empty() || huffman_table.len() > 17 {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 Huffman table must contain 1 to 17 entries".into(),
            ));
        }
        if shift_down.len() != huffman_table.len() {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 Huffman and shift tables have different lengths".into(),
            ));
        }
        for &(bits, symbol) in &huffman_table {
            if bits == 0 || bits > 16 {
                return Err(CodecError::InvalidInput(
                    "Panasonic V8 Huffman symbol length must be 1 to 16 bits".into(),
                ));
            }
            let mask = u16::MAX >> (16 - bits);
            if symbol & !mask != 0 {
                return Err(CodecError::InvalidInput(
                    "Panasonic V8 Huffman symbol exceeds its bit length".into(),
                ));
            }
        }
        if shift_down.iter().any(|shift| *shift > 31) {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 shift-down values must fit five bits".into(),
            ));
        }
        if strips.is_empty() || strips.len() > 64 {
            return Err(CodecError::InvalidInput(
                "Panasonic V8 must contain 1 to 64 strips".into(),
            ));
        }
        for (index, strip) in strips.iter().copied().enumerate() {
            for other in &strips[..index] {
                if rectangles_overlap(strip, *other) {
                    return Err(CodecError::InvalidInput(
                        "Panasonic V8 strips overlap".into(),
                    ));
                }
            }
        }
        Ok(Self {
            gamma_points,
            gamma_slopes,
            clip_value,
            initial_predictors,
            huffman_table,
            shift_down,
            strips,
        })
    }

    pub const fn strip_count(&self) -> usize { self.strips.len() }

    pub fn strips(&self) -> &[PanasonicV8Strip] { &self.strips }
}

const fn rectangles_overlap(left: PanasonicV8Strip, right: PanasonicV8Strip) -> bool {
    left.x < right.x + right.width
        && right.x < left.x + left.width
        && left.y < right.y + right.height
        && right.y < left.y + left.height
}
