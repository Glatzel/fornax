use std::sync::Arc;

use num_enum::TryFromPrimitive;

use crate::{ImgdataPointer, LibrawError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TryFromPrimitive)]
#[repr(i32)]
pub enum ImageSizesFlip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 6,
    CW90 = 9,
}

/// # references
/// - [libraw_image_sizes_t](https://www.libraw.org/docs/API-datastruct-eng.html#libraw_image_sizes_t)
#[derive(Debug, Clone)]
pub struct ImageSizes {
    imgdata: Arc<ImgdataPointer>,
}
impl ImageSizes {
    pub(crate) fn new(imgdata: Arc<ImgdataPointer>) -> Result<Self, LibrawError> {
        Ok(Self { imgdata })
    }
    ///Full size of RAW image (including the frame) in pixels.
    pub fn raw_height(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.raw_height }
    /// Full size of RAW image (including the frame) in pixels.
    pub fn raw_width(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.raw_width }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn height(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.height }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn width(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.width }
    ///Coordinates of the top left corner of the frame (the second corner is
    /// calculated from the full size of the image and size of its visible
    /// part).
    pub fn top_margin(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.top_margin }
    ///Coordinates of the top left corner of the frame (the second corner is
    /// calculated from the full size of the image and size of its visible
    /// part).
    pub fn left_margin(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.left_margin }
    ///Size of the output image (may differ from height/width for cameras that
    /// require image rotation or have non-square pixels).
    pub fn iheight(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.iheight }
    ///Size of the output image (may differ from height/width for cameras that
    /// require image rotation or have non-square pixels).
    pub fn iwidth(&self) -> u16 { (unsafe { *self.imgdata.0 }).sizes.iwidth }
    ///Full size of raw data row in bytes .
    pub fn raw_pitch(&self) -> u32 { (unsafe { *self.imgdata.0 }).sizes.raw_pitch }
    /// Pixel width/height ratio. If it is not unity, scaling of the image along
    /// one of the axes is required during output.
    pub fn pixel_aspect(&self) -> f64 { (unsafe { *self.imgdata.0 }).sizes.pixel_aspect }
    ///Image orientation (0 if does not require rotation; 3 if requires 180-deg
    /// rotation; 5 if 90 deg counterclockwise, 6 if 90 deg clockwise).
    pub fn flip(&self) -> Result<ImageSizesFlip, LibrawError> {
        ImageSizesFlip::try_from((unsafe { *self.imgdata.0 }).sizes.flip).map_err(LibrawError::from)
    }
}
