use std::ffi::{CStr, c_char};
use std::sync::Arc;

use crate::{ImgdataPointer, LibrawError};

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IParamsColorDesc {
    RGBG,
    RGBE,
    GMCY,
    GBTG,
}
impl TryFrom<&CStr> for IParamsColorDesc {
    type Error = crate::LibrawError;
    fn try_from(value: &CStr) -> Result<Self, Self::Error> {
        Ok(match value.to_str()? {
            "RGBG" => IParamsColorDesc::RGBG,
            "RGBE" => IParamsColorDesc::RGBE,
            "GMCY" => IParamsColorDesc::GMCY,
            "GBTG" => IParamsColorDesc::GBTG,
            _ => panic!("Unknown color description."),
        })
    }
}
///# References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_iparams_t>
#[derive(Debug, Clone)]
pub struct IParams {
    imgdata: Arc<ImgdataPointer>,
}
impl IParams {
    pub(crate) fn new(imgdata: Arc<ImgdataPointer>) -> Result<Self, LibrawError> {
        Ok(Self { imgdata })
    }
    ///Camera manufacturer.
    pub fn make(&self) -> &CStr { unsafe { CStr::from_ptr((**self.imgdata).idata.make.as_ptr()) } }
    ///Camera model.
    pub fn model(&self) -> &CStr {
        unsafe { CStr::from_ptr((**self.imgdata).idata.model.as_ptr()) }
    }
    ///There is a huge number of identical cameras sold under different names,
    /// depending on the market (e.g. multiple Panasonic or Canon models)
    /// and even some identical cameras sold under different brands
    /// (Panasonic -> Leica, Sony -> Hasselblad). normalized_make contains
    /// primary vendor name (e.g. Panasonic for Leica re-branded cameras).
    pub fn normalized_make(&self) -> &CStr {
        unsafe { CStr::from_ptr((**self.imgdata).idata.normalized_make.as_ptr()) }
    }
    ///Primary camera model name.
    pub fn normalized_model(&self) -> &CStr {
        unsafe { CStr::from_ptr((**self.imgdata).idata.normalized_model.as_ptr()) }
    }
    ///Primary vendor name in indexed form (enum LibRaw_cameramaker_index,
    /// LIBRAW_CAMERAMAKER_* constant)
    pub fn maker_index(&self) -> u32 { unsafe { (**self.imgdata).idata.maker_index } }
    ///Softwary name/version (mostly for DNG files, to distinguish in-camera
    /// DNGs from Adobe DNG Converter produced ones).
    pub fn software(&self) -> &CStr {
        unsafe { CStr::from_ptr((**self.imgdata).idata.software.as_ptr()) }
    }
    ///   Number of RAW images in file (0 means that the file has not been
    /// recognized).
    pub fn raw_count(&self) -> u32 { unsafe { (**self.imgdata).idata.raw_count } }
    ///Nonzero for Sigma Foveon images
    pub fn is_foveon(&self) -> bool { unsafe { (**self.imgdata).idata.is_foveon != 0 } }
    ///DNG version (for the DNG format).
    pub fn dng_version(&self) -> u32 { unsafe { (**self.imgdata).idata.dng_version } }
    ///  Number of colors in the file.
    pub fn colors(&self) -> i32 { unsafe { (**self.imgdata).idata.colors } }
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
    pub fn filters(&self) -> u32 { unsafe { (**self.imgdata).idata.filters } }
    ///These matrices contains Fuji X-Trans row/col to color mapping. First one
    /// is relative to visible area, while second is positioned relative to
    /// sensor edges.
    pub fn xtrans(&self) -> &[[c_char; 6]; 6] { unsafe { &(**self.imgdata).idata.xtrans } }
    ///These matrices contains Fuji X-Trans row/col to color mapping. First one
    /// is relative to visible area, while second is positioned relative to
    /// sensor edges.
    pub fn xtrans_abs(&self) -> &[[c_char; 6]; 6] { unsafe { &(**self.imgdata).idata.xtrans_abs } }
    ///Description of colors numbered from 0 to 3 (RGBG,RGBE,GMCY, or GBTG).
    pub fn cdesc(&self) -> Result<IParamsColorDesc, LibrawError> {
        IParamsColorDesc::try_from(unsafe { CStr::from_ptr((**self.imgdata).idata.cdesc.as_ptr()) })
    }
    ///XMP packed data length and pointer to extracted XMP packet.
    pub fn xmplen(&self) -> u32 { unsafe { (**self.imgdata).idata.xmplen } }
    ///XMP packed data length and pointer to extracted XMP packet.
    pub fn xmpdata(&self) -> &CStr { unsafe { CStr::from_ptr((**self.imgdata).idata.xmpdata) } }
}
