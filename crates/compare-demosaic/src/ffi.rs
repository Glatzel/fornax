use fornax_core::{BayerChannel, BayerPattern};

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    InvalidDimensions = 1,
    DimensionOverflow = 2,
    AllocationFailed = 3,
    NullPointer = 4,
    OutOfMemory = 5,
    Exception = 6,
    UnknownAlgorithm = 7,
}

impl Status {
    fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::InvalidDimensions,
            2 => Self::DimensionOverflow,
            3 => Self::AllocationFailed,
            4 => Self::NullPointer,
            5 => Self::OutOfMemory,
            6 => Self::Exception,
            7 => Self::UnknownAlgorithm,
            other => panic!("unknown compare-demosaic FFI status {other}"),
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::InvalidDimensions => "LibRaw reference requires dimensions of at least 3x3",
            Self::DimensionOverflow => "LibRaw reference dimensions exceed supported limits",
            Self::AllocationFailed => "LibRaw reference image allocation failed",
            Self::NullPointer => "LibRaw reference received a null buffer",
            Self::OutOfMemory => "LibRaw reference ran out of memory",
            Self::Exception => "LibRaw reference raised an unknown exception",
            Self::UnknownAlgorithm => "LibRaw reference returned unknown algorithm",
        }
    }
}

unsafe extern "C" {
    fn compare_demosaic_libraw(
        algorithm: u32,
        mosaic: *const u16,
        width: u32,
        height: u32,
        filters: u32,
        output: *mut u16,
    ) -> i32;
}

pub fn run(
    algorithm: u32,
    codes: &[u16],
    width: u32,
    height: u32,
    filters: u32,
) -> mischief::Result<Vec<u16>> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions overflow"))?;
    if codes.len() != pixels {
        return Err(mischief::mischief!(
            "synthetic image contains {} samples, expected {pixels}",
            codes.len()
        ));
    }
    let samples = pixels
        .checked_mul(3)
        .ok_or_else(|| mischief::mischief!("RGB output dimensions overflow"))?;
    let mut output = vec![0_u16; samples];

    // SAFETY: both buffers are valid for the dimensions checked above, and the
    // C++ function writes exactly `pixels * 3` samples.
    let code = unsafe {
        compare_demosaic_libraw(
            algorithm,
            codes.as_ptr(),
            width,
            height,
            filters,
            output.as_mut_ptr(),
        )
    };
    let status = Status::from_code(code);
    if status == Status::Ok {
        Ok(output)
    } else {
        Err(mischief::mischief!("{}", status.message()))
    }
}

pub fn libraw_filters(pattern: BayerPattern) -> u32 {
    let mut filters = 0_u32;
    for y in 0..8 {
        for x in 0..2 {
            let code = match pattern.channel_at(x, y) {
                BayerChannel::R => 0_u32,
                BayerChannel::GRedRow => 1,
                BayerChannel::B => 2,
                BayerChannel::GBlueRow => 3,
            };
            filters |= code << ((y * 2 + x) * 2);
        }
    }
    filters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_masks_match_all_bayer_tiles() {
        for pattern in BayerPattern::ALL {
            let filters = libraw_filters(pattern);
            for y in 0..8 {
                for x in 0..2 {
                    let shift = (y * 2 + x) * 2;
                    let code = (filters >> shift) & 3;
                    let channel = match code {
                        0 => BayerChannel::R,
                        1 => BayerChannel::GRedRow,
                        2 => BayerChannel::B,
                        _ => BayerChannel::GBlueRow,
                    };
                    assert_eq!(channel, pattern.channel_at(x, y));
                }
            }
        }
    }

    #[test]
    fn reference_rejects_invalid_and_overflowing_dimensions() {
        let mosaic = [0_u16; 9];
        let mut output = [0_u16; 27];
        for (width, height, expected) in [
            (2, 3, Status::InvalidDimensions),
            (3, 2, Status::InvalidDimensions),
            (65_536, 3, Status::DimensionOverflow),
            (3, 65_536, Status::DimensionOverflow),
        ] {
            // SAFETY: the C++ function rejects these dimensions before reading
            // either buffer, and both pointers remain valid for the call.
            let code = unsafe {
                compare_demosaic_libraw(
                    0,
                    mosaic.as_ptr(),
                    width,
                    height,
                    libraw_filters(BayerPattern::RGGB),
                    output.as_mut_ptr(),
                )
            };
            assert_eq!(Status::from_code(code), expected);
        }
    }
}
