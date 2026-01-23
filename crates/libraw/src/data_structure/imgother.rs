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
    pub(crate) fn new(imgdata: Arc<ImgdataPtr>) -> Self {
        Self {
            arc_imgdata_ptr: imgdata,
        }
    }
    pub fn latitude(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.latitude }
    }
    pub fn longitude(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.longitude }
    }
    pub fn gps_time_stamp(&self) -> [f32; 3usize] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).other.parsed_gps.gpstimestamp }
    }
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
    pub(crate) fn new(imgdata: Arc<ImgdataPtr>) -> Result<Self, LibrawError> {
        Ok(Self { imgdata })
    }
    ///ISO sensitivity.
    pub fn iso_speed(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.iso_speed } }
    ///Shutter speed.
    pub fn shutter(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.shutter } }
    ///Aperture.
    pub fn aperture(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.aperture } }
    ///Focal length.
    pub fn focal_len(&self) -> f32 { unsafe { (*self.imgdata.ptr()).other.focal_len } }
    ///Date of shooting.
    pub fn timestamp(&self) -> DateTime<Utc> {
        unsafe { DateTime::from_timestamp((*self.imgdata.ptr()).other.timestamp, 0).unwrap() }
    }
    ///Serial number of image.
    pub fn shot_order(&self) -> u32 { unsafe { (*self.imgdata.ptr()).other.shot_order } }
    ///GPS data (unparsed block, to write to output as is).
    pub fn gpsdata(&self) -> [u32; 32] { unsafe { (*self.imgdata.ptr()).other.gpsdata } }
    ///Parsed GPS-data: longitude/latitude/altitude and time stamp.
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
