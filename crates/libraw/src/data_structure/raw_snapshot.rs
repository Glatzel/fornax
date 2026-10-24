use std::ffi::CStr;

use crate::extension::error_handling::check_raw_alloc;
use crate::{Libraw, LibrawError, LibrawErrorCode};

/// A compact copy of the unpacked LibRaw sensor buffer.
#[derive(Clone, Debug, PartialEq)]
pub enum UnpackedRawBuffer {
    /// Single-plane unsigned integer samples.
    Integer(Vec<u16>),
    /// Single-plane floating-point samples.
    Float(Vec<f32>),
}

/// A safe, owned snapshot of LibRaw metadata and unpacked raw samples.
///
/// The snapshot contains no LibRaw pointers and remains valid after the owning
/// [`Libraw`] value is dropped.
#[derive(Clone, Debug, PartialEq)]
pub struct UnpackedRawSnapshot {
    /// Full stored raster height.
    pub raw_height: u32,
    /// Full stored raster width.
    pub raw_width: u32,
    /// Full stored raster row stride in bytes.
    pub raw_pitch: u32,
    /// Active image height.
    pub height: u32,
    /// Active image width.
    pub width: u32,
    /// Active image top origin in the stored raster.
    pub top_margin: u32,
    /// Active image left origin in the stored raster.
    pub left_margin: u32,
    /// LibRaw orientation code.
    pub flip: i32,
    /// Camera make.
    pub make: String,
    /// Camera model.
    pub model: String,
    /// Normalized camera make.
    pub normalized_make: String,
    /// Normalized camera model.
    pub normalized_model: String,
    /// CFA color description.
    pub cdesc: String,
    /// Number of RAW images in the container.
    pub raw_count: u32,
    /// Whether LibRaw identified a Foveon sensor.
    pub is_foveon: bool,
    /// DNG version, or zero for a non-DNG source.
    pub dng_version: u32,
    /// Number of declared image colors.
    pub colors: i32,
    /// Packed LibRaw CFA descriptor.
    pub filters: u32,
    /// Source sample precision in bits.
    pub raw_bps: u32,
    /// Common black level in stored sample codes.
    pub black: u32,
    /// First six canonical black entries: four color offsets and repeat shape.
    pub cblack: [u32; 6],
    /// Decoder processing maximum.
    pub maximum: u32,
    /// Measured data maximum.
    pub data_maximum: u32,
    /// Per-color linear maxima.
    pub linear_max: [u32; 4],
    /// DNG-declared white levels.
    pub dng_whitelevel: [u32; 4],
    /// DNG default crop origin and size as `[x, y, width, height]`.
    pub default_crop: [u16; 4],
    /// Whether a three-channel working image is present.
    pub has_color3_image: bool,
    /// Whether a four-channel working image is present.
    pub has_color4_image: bool,
    /// Whether a three-channel float working image is present.
    pub has_float3_image: bool,
    /// Whether a four-channel float working image is present.
    pub has_float4_image: bool,
    /// LibRaw progress flags at snapshot time.
    pub progress_flags: u32,
    /// Name of the LibRaw unpack function when available.
    pub unpack_function: Option<String>,
    /// Single-plane unpacked sensor samples.
    pub buffer: Option<UnpackedRawBuffer>,
}

impl Libraw {
    /// Copies unpacked LibRaw metadata and the complete stored sensor raster.
    ///
    /// # Errors
    ///
    /// Returns a LibRaw or allocation error when the unpacked buffer is absent
    /// or its geometry and row stride cannot describe a contiguous allocation.
    pub fn unpacked_snapshot(&self) -> Result<UnpackedRawSnapshot, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());

        // SAFETY: `raw_alloc` was checked above, so LibRaw owns a valid data
        // structure for the duration of this immutable snapshot.
        let data = unsafe { &*self.imgdata_ptr() };
        let sizes = data.sizes;
        let idata = data.idata;
        let color = data.color;
        let rawdata = data.rawdata;

        let buffer = if !rawdata.raw_image.is_null() {
            Some(UnpackedRawBuffer::Integer(copy_integer_rows(
                rawdata.raw_image,
                sizes.raw_width,
                sizes.raw_height,
                sizes.raw_pitch,
            )?))
        } else if !rawdata.float_image.is_null() {
            Some(UnpackedRawBuffer::Float(copy_float_rows(
                rawdata.float_image,
                sizes.raw_width,
                sizes.raw_height,
                sizes.raw_pitch,
            )?))
        } else {
            None
        };

        let unpack_function = unsafe {
            let name = libraw_sys::libraw_unpack_function_name(self.imgdata_ptr());
            (!name.is_null()).then(|| CStr::from_ptr(name).to_string_lossy().into_owned())
        };

        Ok(UnpackedRawSnapshot {
            raw_height: u32::from(sizes.raw_height),
            raw_width: u32::from(sizes.raw_width),
            raw_pitch: sizes.raw_pitch,
            height: u32::from(sizes.height),
            width: u32::from(sizes.width),
            top_margin: u32::from(sizes.top_margin),
            left_margin: u32::from(sizes.left_margin),
            flip: sizes.flip,
            make: c_array_string(&idata.make),
            model: c_array_string(&idata.model),
            normalized_make: c_array_string(&idata.normalized_make),
            normalized_model: c_array_string(&idata.normalized_model),
            cdesc: c_array_string(&idata.cdesc),
            raw_count: idata.raw_count,
            is_foveon: idata.is_foveon != 0,
            dng_version: idata.dng_version,
            colors: idata.colors,
            filters: idata.filters,
            raw_bps: color.raw_bps,
            black: color.black,
            cblack: [
                color.cblack[0],
                color.cblack[1],
                color.cblack[2],
                color.cblack[3],
                color.cblack[4],
                color.cblack[5],
            ],
            maximum: color.maximum,
            data_maximum: color.data_maximum,
            linear_max: color.linear_max,
            dng_whitelevel: color.dng_levels.dng_whitelevel,
            default_crop: color.dng_levels.default_crop,
            has_color3_image: !rawdata.color3_image.is_null(),
            has_color4_image: !rawdata.color4_image.is_null(),
            has_float3_image: !rawdata.float3_image.is_null(),
            has_float4_image: !rawdata.float4_image.is_null(),
            progress_flags: data.progress_flags,
            unpack_function,
            buffer,
        })
    }
}

fn copy_integer_rows(
    source: *const u16,
    width: u16,
    height: u16,
    pitch: u32,
) -> Result<Vec<u16>, LibrawError> {
    let row_bytes = u32::from(width)
        .checked_mul(size_of::<u16>() as u32)
        .ok_or_else(geometry_error)?;
    if pitch < row_bytes || !pitch.is_multiple_of(size_of::<u16>() as u32) {
        return Err(geometry_error());
    }
    let stride = usize::try_from(pitch / size_of::<u16>() as u32).map_err(|_| geometry_error())?;
    let sample_count = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(geometry_error)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| allocation_error())?;
    for row in 0..usize::from(height) {
        // SAFETY: LibRaw reports `height` rows each addressable by `pitch`
        // bytes. The validated row width and stride keep every read inside
        // that allocation.
        let row_samples =
            unsafe { std::slice::from_raw_parts(source.add(row * stride), width.into()) };
        samples.extend_from_slice(row_samples);
    }
    Ok(samples)
}

fn copy_float_rows(
    source: *const f32,
    width: u16,
    height: u16,
    pitch: u32,
) -> Result<Vec<f32>, LibrawError> {
    let row_bytes = u32::from(width)
        .checked_mul(size_of::<f32>() as u32)
        .ok_or_else(geometry_error)?;
    if pitch < row_bytes || !pitch.is_multiple_of(size_of::<f32>() as u32) {
        return Err(geometry_error());
    }
    let stride = usize::try_from(pitch / size_of::<f32>() as u32).map_err(|_| geometry_error())?;
    let sample_count = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(geometry_error)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| allocation_error())?;
    for row in 0..usize::from(height) {
        // SAFETY: LibRaw reports `height` rows each addressable by `pitch`
        // bytes. The validated row width and stride keep every read inside
        // that allocation.
        let row_samples =
            unsafe { std::slice::from_raw_parts(source.add(row * stride), width.into()) };
        samples.extend_from_slice(row_samples);
    }
    Ok(samples)
}

fn c_array_string<const N: usize>(value: &[libc::c_char; N]) -> String {
    let length = value.iter().position(|byte| *byte == 0).unwrap_or(N);
    let bytes = value[..length]
        .iter()
        .map(|byte| *byte as u8)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn geometry_error() -> LibrawError {
    LibrawError::Libraw {
        code: LibrawErrorCode::DataError,
        message: "unpacked buffer geometry is inconsistent".into(),
    }
}

fn allocation_error() -> LibrawError {
    LibrawError::Libraw {
        code: LibrawErrorCode::UnsufficientMemory,
        message: "could not allocate the unpacked buffer snapshot".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_the_cr2_fixture_without_post_processing() -> mischief::Result<()> {
        let context = Libraw::default();
        context.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let snapshot = context.unpacked_snapshot()?;

        assert_eq!(snapshot.raw_width, 5_360);
        assert_eq!(snapshot.raw_height, 3_516);
        assert_eq!(snapshot.raw_pitch, 10_720);
        assert_eq!(snapshot.width, 5_202);
        assert_eq!(snapshot.height, 3_464);
        assert_eq!(snapshot.top_margin, 52);
        assert_eq!(snapshot.left_margin, 158);
        assert_eq!(snapshot.make, "Canon");
        assert_eq!(snapshot.model, "EOS 7D");
        assert!(snapshot.raw_bps > 0);
        assert_eq!(snapshot.black, 2_048);
        assert_eq!(&snapshot.cblack[..4], &[0, 0, 0, 0]);
        assert_eq!(&snapshot.cblack[4..], &[0, 0]);
        assert_eq!(snapshot.progress_flags & (1 << 4), 0);
        let UnpackedRawBuffer::Integer(samples) = snapshot.buffer.unwrap() else {
            panic!("Canon CR2 raw_image is expected to use u16 storage");
        };
        assert_eq!(
            samples.len(),
            usize::try_from(u64::from(snapshot.raw_width) * u64::from(snapshot.raw_height))
                .unwrap()
        );
        Ok(())
    }
}
