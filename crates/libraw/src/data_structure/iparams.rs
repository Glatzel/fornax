use libraw_sys as sys;

use crate::utils::c_char_to_string;
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Copy, Debug)]
pub enum ColorDesc {
    RGBG,
    RGBE,
    GMCY,
    GBTG,
}
impl From<&str> for ColorDesc {
    fn from(value: &str) -> Self {
        match value {
            "RGBG" => ColorDesc::RGBG,
            "RGBE" => ColorDesc::RGBE,
            "GMCY" => ColorDesc::GMCY,
            "GBTG" => ColorDesc::GBTG,
            _ => panic!("Unknown color description."),
        }
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct LibrawIParams {
    make: String,
    model: String,
    normalized_make: String,
    normalized_model: String,
    maker_index: u32,
    software: String,
    raw_count: u32,
    is_foveon: bool,
    dng_version: u32,
    colors: i32,
    filters: u32,
    xtrans: [[i8; 6]; 6],
    xtrans_abs: [[i8; 6]; 6],
    cdesc: ColorDesc,
    xmplen: u32,
    xmpdata: String,
}
impl LibrawIParams {
    pub(crate) fn new(imgdata: *mut sys::libraw_data_t) -> miette::Result<Self> {
        let imgdata = unsafe { *imgdata };
        Ok(Self {
            make: c_char_to_string(imgdata.idata.make.as_ptr()),
            model: c_char_to_string(imgdata.idata.model.as_ptr()),
            normalized_make: c_char_to_string(imgdata.idata.normalized_make.as_ptr()),
            normalized_model: c_char_to_string(imgdata.idata.normalized_model.as_ptr()),
            maker_index: imgdata.idata.maker_index,
            software: c_char_to_string(imgdata.idata.software.as_ptr()),
            raw_count: imgdata.idata.raw_count,
            is_foveon: imgdata.idata.is_foveon != 0,
            dng_version: imgdata.idata.dng_version,
            colors: imgdata.idata.colors,
            filters: imgdata.idata.filters,
            xtrans: imgdata.idata.xtrans,
            xtrans_abs: imgdata.idata.xtrans_abs,
            cdesc: ColorDesc::from(c_char_to_string(imgdata.idata.cdesc.as_ptr()).as_str()),
            xmplen: imgdata.idata.xmplen,
            xmpdata: unsafe {
                std::ffi::CStr::from_ptr(imgdata.idata.xmpdata)
                    .to_str()
                    .unwrap()
                    .to_string()
            },
        })
    }
    ///Camera manufacturer.
    pub fn make(&self) -> String { self.make.clone() }
    ///Camera model.
    pub fn model(&self) -> String { self.model.clone() }
    ///There is a huge number of identical cameras sold under different names,
    /// depending on the market (e.g. multiple Panasonic or Canon models)
    /// and even some identical cameras sold under different brands
    /// (Panasonic -> Leica, Sony -> Hasselblad). normalized_make contains
    /// primary vendor name (e.g. Panasonic for Leica re-branded cameras).
    pub fn normalized_make(&self) -> String { self.normalized_make.clone() }
    ///Primary camera model name.
    pub fn normalized_model(&self) -> String { self.normalized_model.clone() }
    ///Primary vendor name in indexed form (enum LibRaw_cameramaker_index,
    /// LIBRAW_CAMERAMAKER_* constant)
    pub fn maker_index(&self) -> u32 { self.maker_index }
    ///Softwary name/version (mostly for DNG files, to distinguish in-camera
    /// DNGs from Adobe DNG Converter produced ones).
    pub fn software(&self) -> String { self.software.clone() }
    ///   Number of RAW images in file (0 means that the file has not been
    /// recognized).
    pub fn raw_count(&self) -> u32 { self.raw_count }
    ///Nonzero for Sigma Foveon images
    pub fn is_foveon(&self) -> bool { self.is_foveon }
    ///DNG version (for the DNG format).
    pub fn dng_version(&self) -> u32 { self.dng_version }
    ///  Number of colors in the file.
    pub fn colors(&self) -> i32 { self.colors }
    ///Bit mask describing the order of color pixels in the matrix (0 for
    /// full-color images). 32 bits of this field describe 16 pixels (8 rows
    /// with two pixels in each, from left to right and from top to bottom).
    /// Each two bits have values 0 to 3, which correspond to four possible
    /// colors. Convenient work with this field is ensured by the
    /// COLOR(row,column) function, which returns the number of the active
    /// color for a given pixel.
    ///
    ///Values less than 1000 are reserved as special cases:
    /// - 1 - Leaf Catchlight with 16x16 bayer matrix;
    /// - 9 - Fuji X-Trans (6x6 matrix)
    /// - 3..8 and 10..999 - are unused.
    pub fn filters(&self) -> u32 { self.filters }
    ///These matrices contains Fuji X-Trans row/col to color mapping. First one
    /// is relative to visible area, while second is positioned relative to
    /// sensor edges.
    pub fn xtrans(&self) -> [[i8; 6]; 6] { self.xtrans }
    ///These matrices contains Fuji X-Trans row/col to color mapping. First one
    /// is relative to visible area, while second is positioned relative to
    /// sensor edges.
    pub fn xtrans_abs(&self) -> [[i8; 6]; 6] { self.xtrans_abs }
    ///Description of colors numbered from 0 to 3 (RGBG,RGBE,GMCY, or GBTG).
    pub fn cdesc(&self) -> ColorDesc { self.cdesc }
    ///XMP packed data length and pointer to extracted XMP packet.
    pub fn xmplen(&self) -> u32 { self.xmplen }
    ///XMP packed data length and pointer to extracted XMP packet.
    pub fn xmpdata(&self) -> String { self.xmpdata.clone() }
}
