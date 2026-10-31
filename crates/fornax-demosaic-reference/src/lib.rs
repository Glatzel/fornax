//! Exact-reference demosaic backend.
//!
//! This private workspace crate is the licensing boundary for demosaic
//! implementations adapted from LibRaw and darktable. See the crate README.

use std::sync::Mutex;

/// A reference-backed demosaic implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Algorithm {
    Vng = 0,
    Dcb = 1,
    Dht = 2,
    Aahd = 3,
    Amaze = 4,
    Lmmse = 5,
}

/// Error returned by the reference backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidDimensions,
    DimensionOverflow,
    AllocationFailed,
    NullPointer,
    OutOfMemory,
    Exception,
    UnknownAlgorithm,
}

unsafe extern "C" {
    fn fornax_demosaic_reference(
        algorithm: u32,
        mosaic: *const u16,
        width: u32,
        height: u32,
        filters: u32,
        output: *mut u16,
    ) -> i32;
}

#[derive(Default)]
struct BackendState {
    last: Option<CacheEntry>,
}

struct CacheEntry {
    algorithm: Algorithm,
    mosaic: Vec<u16>,
    width: u32,
    height: u32,
    filters: u32,
    output: Vec<u16>,
}

static BACKEND_STATE: Mutex<BackendState> = Mutex::new(BackendState { last: None });

/// Demosaics one Bayer mosaic into interleaved 16-bit RGB.
///
/// `filters` uses the LibRaw four-row Bayer mask representation.
pub fn demosaic(
    algorithm: Algorithm,
    mosaic: &[u16],
    width: u32,
    height: u32,
    filters: u32,
) -> Result<Vec<u16>, Error> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or(Error::DimensionOverflow)?;
    if mosaic.len() != pixels {
        return Err(Error::InvalidDimensions);
    }
    let samples = pixels.checked_mul(3).ok_or(Error::DimensionOverflow)?;
    let mut state = BACKEND_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(cached) = &state.last
        && cached.algorithm == algorithm
        && cached.mosaic == mosaic
        && cached.width == width
        && cached.height == height
        && cached.filters == filters
    {
        return Ok(cached.output.clone());
    }
    let mut output = vec![0_u16; samples];

    // SAFETY: both buffers are valid for `pixels`, and the C++ backend writes
    // exactly `pixels * 3` samples on success.
    let status = unsafe {
        fornax_demosaic_reference(
            algorithm as u32,
            mosaic.as_ptr(),
            width,
            height,
            filters,
            output.as_mut_ptr(),
        )
    };
    if status == 0 {
        state.last = Some(CacheEntry {
            algorithm,
            mosaic: mosaic.to_vec(),
            width,
            height,
            filters,
            output: output.clone(),
        });
        Ok(output)
    } else {
        Err(match status {
            1 => Error::InvalidDimensions,
            2 => Error::DimensionOverflow,
            3 => Error::AllocationFailed,
            4 => Error::NullPointer,
            5 => Error::OutOfMemory,
            6 => Error::Exception,
            7 => Error::UnknownAlgorithm,
            _ => Error::Exception,
        })
    }
}
