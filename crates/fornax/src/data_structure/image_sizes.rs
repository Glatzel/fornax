use libraw_sys as sys;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flip {
    None = 0,
    Rotate180 = 3,
    CCW90 = 6,
    CW90 = 9,
}
impl From<i32> for Flip {
    fn from(value: i32) -> Self {
        match value {
            0 => Flip::None,
            3 => Flip::Rotate180,
            6 => Flip::CCW90,
            9 => Flip::CW90,
            _ => panic!("Invalid value for MyEnum"),
        }
    }
}
/// # references
/// - https://www.libraw.org/docs/API-datastruct-eng.html#libraw_image_sizes_t
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub struct ImageSizes {
    ///Full size of RAW image (including the frame) in pixels.
    pub raw_height: u16,
    /// Full size of RAW image (including the frame) in pixels.
    pub raw_width: u16,
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub height: u16,
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub width: u16,
    ///Coordinates of the top left corner of the frame (the second corner is calculated from the
    /// full size of the image and size of its visible part).
    pub top_margin: u16,
    ///Coordinates of the top left corner of the frame (the second corner is calculated from the
    /// full size of the image and size of its visible part).
    pub left_margin: u16,
    ///Size of the output image (may differ from height/width for cameras that require image
    /// rotation or have non-square pixels).
    pub iheight: u16,
    ///Size of the output image (may differ from height/width for cameras that require image
    /// rotation or have non-square pixels).
    pub iwidth: u16,
    ///Full size of raw data row in bytes .
    pub raw_pitch: u32,
    /// Pixel width/height ratio. If it is not unity, scaling of the image along one of the axes is required during output.
    pub pixel_aspect: f64,
    ///Image orientation (0 if does not require rotation; 3 if requires 180-deg rotation; 5 if 90 deg counterclockwise, 6 if 90 deg clockwise).
    pub flip: Flip,
}

impl ImageSizes {
    pub(crate) fn new(sizes: &sys::libraw_image_sizes_t) -> Self {
        Self {
            raw_height: sizes.raw_height,
            raw_width: sizes.raw_width,
            height: sizes.height,
            width: sizes.width,
            top_margin: sizes.top_margin,
            left_margin: sizes.left_margin,
            iheight: sizes.iheight,
            iwidth: sizes.iwidth,
            raw_pitch: sizes.raw_pitch,
            pixel_aspect: sizes.pixel_aspect,
            flip: Flip::from(sizes.flip),
        }
    }
}
