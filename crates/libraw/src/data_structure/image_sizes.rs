use libraw_sys as sys;
use num_enum::TryFromPrimitive;

use crate::LibrawError;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive)]
#[repr(i32)]
pub enum ImageSizesFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 6,
    CW90 = 9,
}

/// # references
/// - [libraw_image_sizes_t](https://www.libraw.org/docs/API-datastruct-eng.html#libraw_image_sizes_t)
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Copy, Clone)]
pub struct ImageSizes {
    raw_height: u16,
    raw_width: u16,
    height: u16,
    width: u16,
    top_margin: u16,
    left_margin: u16,
    iheight: u16,
    iwidth: u16,
    raw_pitch: u32,
    pixel_aspect: f64,
    flip: ImageSizesFlip,
}

impl ImageSizes {
    pub(crate) fn new(imgdata: *mut sys::libraw_data_t) -> Result<Self, LibrawError> {
        let imgdata = unsafe { *imgdata };
        Ok(Self {
            raw_height: imgdata.sizes.raw_height,
            raw_width: imgdata.sizes.raw_width,
            height: imgdata.sizes.height,
            width: imgdata.sizes.width,
            top_margin: imgdata.sizes.top_margin,
            left_margin: imgdata.sizes.left_margin,
            iheight: imgdata.sizes.iheight,
            iwidth: imgdata.sizes.iwidth,
            raw_pitch: imgdata.sizes.raw_pitch,
            pixel_aspect: imgdata.sizes.pixel_aspect,
            flip: ImageSizesFlip::try_from(imgdata.sizes.flip).map_err(LibrawError::from)?,
        })
    }
    ///Full size of RAW image (including the frame) in pixels.
    pub fn raw_height(&self) -> u16 { self.raw_height }
    /// Full size of RAW image (including the frame) in pixels.
    pub fn raw_width(&self) -> u16 { self.raw_width }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn height(&self) -> u16 { self.height }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn width(&self) -> u16 { self.width }
    ///Coordinates of the top left corner of the frame (the second corner is
    /// calculated from the full size of the image and size of its visible
    /// part).
    pub fn top_margin(&self) -> u16 { self.top_margin }
    ///Coordinates of the top left corner of the frame (the second corner is
    /// calculated from the full size of the image and size of its visible
    /// part).
    pub fn left_margin(&self) -> u16 { self.left_margin }
    ///Size of the output image (may differ from height/width for cameras that
    /// require image rotation or have non-square pixels).
    pub fn iheight(&self) -> u16 { self.iheight }
    ///Size of the output image (may differ from height/width for cameras that
    /// require image rotation or have non-square pixels).
    pub fn iwidth(&self) -> u16 { self.iwidth }
    ///Full size of raw data row in bytes .
    pub fn raw_pitch(&self) -> u32 { self.raw_pitch }
    /// Pixel width/height ratio. If it is not unity, scaling of the image along
    /// one of the axes is required during output.
    pub fn pixel_aspect(&self) -> f64 { self.pixel_aspect }
    ///Image orientation (0 if does not require rotation; 3 if requires 180-deg
    /// rotation; 5 if 90 deg counterclockwise, 6 if 90 deg clockwise).
    pub fn flip(&self) -> ImageSizesFlip { self.flip }
}
