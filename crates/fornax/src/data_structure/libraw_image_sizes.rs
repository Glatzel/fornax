use libraw_sys as sys;

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
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
pub struct LibrawImageSizes {
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
    flip: Flip,
}

impl LibrawImageSizes {
    pub(crate) fn new(imgdata: *mut sys::libraw_data_t) -> miette::Result<Self> {
        if unsafe { (*imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("")
        }
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
            flip: Flip::from(imgdata.sizes.flip),
        })
    }
    ///Full size of RAW image (including the frame) in pixels.
    pub fn raw_height(&self) -> u16 {
        self.raw_height
    }
    /// Full size of RAW image (including the frame) in pixels.
    pub fn raw_width(&self) -> u16 {
        self.raw_width
    }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn height(&self) -> u16 {
        self.height
    }
    ///Size of visible ("meaningful") part of the image (without the frame).
    pub fn width(&self) -> u16 {
        self.width
    }
    ///Coordinates of the top left corner of the frame (the second corner is calculated from the
    /// full size of the image and size of its visible part).
    pub fn top_margin(&self) -> u16 {
        self.top_margin
    }
    ///Coordinates of the top left corner of the frame (the second corner is calculated from the
    /// full size of the image and size of its visible part).
    pub fn left_margin(&self) -> u16 {
        self.left_margin
    }
    ///Size of the output image (may differ from height/width for cameras that require image
    /// rotation or have non-square pixels).
    pub fn iheight(&self) -> u16 {
        self.iheight
    }
    ///Size of the output image (may differ from height/width for cameras that require image
    /// rotation or have non-square pixels).
    pub fn iwidth(&self) -> u16 {
        self.iwidth
    }
    ///Full size of raw data row in bytes .
    pub fn raw_pitch(&self) -> u32 {
        self.raw_pitch
    }
    /// Pixel width/height ratio. If it is not unity, scaling of the image along one of the axes is
    /// required during output.
    pub fn pixel_aspect(&self) -> f64 {
        self.pixel_aspect
    }
    ///Image orientation (0 if does not require rotation; 3 if requires 180-deg rotation; 5 if 90
    /// deg counterclockwise, 6 if 90 deg clockwise).
    pub fn flip(&self) -> Flip {
        self.flip
    }
}
