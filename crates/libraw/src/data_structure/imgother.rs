use std::ffi::c_char;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use envoy::PtrToString;

use crate::{ImgdataPtr, LibrawError};

#[derive(Debug, Clone)]
pub struct ImgOtherGpsInfo {
    arc_imgdata_ptr: Arc<ImgdataPtr>,
}
impl ImgOtherGpsInfo {
    pub(crate) const fn new(imgdata: Arc<ImgdataPtr>) -> Self {
        Self {
            arc_imgdata_ptr: imgdata,
        }
    }
    #[must_use]
    pub fn latitude(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.latitude }
    }
    #[must_use]
    pub fn longitude(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.longitude }
    }
    #[must_use]
    pub fn gps_time_stamp(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.gpstimestamp }
    }
    #[must_use]
    pub fn altitude(&self) -> f32 {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.altitude }
    }
    pub fn altref(&self) -> Result<String, LibrawError> {
        unsafe {
            Ok(
                ((*self.arc_imgdata_ptr.ptr()).other.parsed_gps.altref as *const c_char)
                    .to_string()?,
            )
        }
    }
    pub fn latref(&self) -> Result<String, LibrawError> {
        unsafe {
            Ok(
                ((*self.arc_imgdata_ptr.ptr()).other.parsed_gps.latref as *const c_char)
                    .to_string()?,
            )
        }
    }
    pub fn longref(&self) -> Result<String, LibrawError> {
        unsafe {
            Ok(
                ((*self.arc_imgdata_ptr.ptr()).other.parsed_gps.longref as *const c_char)
                    .to_string()?,
            )
        }
    }
    pub fn gpsstatus(&self) -> Result<String, LibrawError> {
        unsafe {
            Ok(
                ((*self.arc_imgdata_ptr.ptr()).other.parsed_gps.gpsstatus as *const c_char)
                    .to_string()?,
            )
        }
    }
    pub fn gpsparsed(&self) -> Result<String, LibrawError> {
        unsafe {
            Ok(
                ((*self.arc_imgdata_ptr.ptr()).other.parsed_gps.gpsparsed as *const c_char)
                    .to_string()?,
            )
        }
    }
}
///# References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#libraw_imgother_t>
#[derive(Debug, Clone)]
pub struct ImgOther {
    imgdata: Arc<ImgdataPtr>,
}
impl ImgOther {
    pub(crate) const fn new(imgdata: Arc<ImgdataPtr>) -> Result<Self, LibrawError> {
        Ok(Self { imgdata })
    }
    ///ISO sensitivity.
    #[must_use]
    pub fn iso_speed(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.iso_speed } }
    ///Shutter speed.
    #[must_use]
    pub fn shutter(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.shutter } }
    ///Aperture.
    #[must_use]
    pub fn aperture(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.aperture } }
    ///Focal length.
    #[must_use]
    pub fn focal_len(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.focal_len } }
    ///Date of shooting.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        unsafe { DateTime::from_timestamp((*self.imgdata.ptr()).other.timestamp, 0).unwrap() }
    }
    ///Serial number of image.
    #[must_use]
    pub fn shot_order(&self) -> u32 { unsafe { (*self.imgdata.ptr()).other.shot_order } }
    ///GPS data (unparsed block, to write to output as is).
    #[must_use]
    pub fn gpsdata(&self) -> [u32; 32] { unsafe { (*self.imgdata.ptr()).other.gpsdata } }
    ///Parsed GPS-data: longitude/latitude/altitude and time stamp.
    #[must_use]
    pub fn parsed_gps(&self) -> ImgOtherGpsInfo { ImgOtherGpsInfo::new(self.imgdata.clone()) }
    ///Image description.
    pub fn desc(&self) -> Result<String, LibrawError> {
        unsafe { Ok(((*self.imgdata.ptr()).other.desc.as_ptr()).to_string()?) }
    }
    ///Author of image.
    pub fn artist(&self) -> Result<String, LibrawError> {
        unsafe { Ok(((*self.imgdata.ptr()).other.artist.as_ptr()).to_string()?) }
    }
}
