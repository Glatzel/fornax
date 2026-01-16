use std::ffi::CStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::{ImgdataPointer, LibrawError};

#[derive(Debug, Clone)]
pub struct ImgOtherGpsInfo {
    imgdata: Arc<ImgdataPointer>,
}
impl ImgOtherGpsInfo {
    pub(crate) fn new(imgdata: Arc<ImgdataPointer>) -> Self { Self { imgdata } }
    pub fn latitude(&self) -> [f32; 3usize] {
        unsafe { (*self.imgdata.0).other.parsed_gps.latitude }
    }
    pub fn longitude(&self) -> [f32; 3usize] {
        unsafe { (*self.imgdata.0).other.parsed_gps.longitude }
    }
    pub fn gps_time_stamp(&self) -> [f32; 3usize] {
        unsafe { (*self.imgdata.0).other.parsed_gps.gpstimestamp }
    }
    pub fn altitude(&self) -> f32 { unsafe { (*self.imgdata.0).other.parsed_gps.altitude } }
    pub fn altref(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.parsed_gps.altref as *const i8) }
    }
    pub fn latref(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.parsed_gps.latref as *const i8) }
    }
    pub fn longref(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.parsed_gps.longref as *const i8) }
    }
    pub fn gpsstatus(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.parsed_gps.gpsstatus as *const i8) }
    }
    pub fn gpsparsed(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.parsed_gps.gpsparsed as *const i8) }
    }
}

#[derive(Debug, Clone)]
pub struct ImgOther {
    imgdata: Arc<ImgdataPointer>,
}
impl ImgOther {
    pub(crate) fn new(imgdata: Arc<ImgdataPointer>) -> Result<Self, LibrawError> {
        Ok(Self { imgdata })
    }
    ///ISO sensitivity.
    pub fn iso_speed(&self) -> f32 { unsafe { (*self.imgdata.0).other.iso_speed } }
    ///Shutter speed.
    pub fn shutter(&self) -> f32 { unsafe { (*self.imgdata.0).other.shutter } }
    ///Aperture.
    pub fn aperture(&self) -> f32 { unsafe { (*self.imgdata.0).other.aperture } }
    ///Focal length.
    pub fn focal_len(&self) -> f32 { unsafe { (*self.imgdata.0).other.focal_len } }
    ///Date of shooting.
    pub fn timestamp(&self) -> DateTime<Utc> {
        unsafe { DateTime::from_timestamp((*self.imgdata.0).other.timestamp, 0).unwrap() }
    }
    ///Serial number of image.
    pub fn shot_order(&self) -> u32 { unsafe { (*self.imgdata.0).other.shot_order } }
    ///GPS data (unparsed block, to write to output as is).
    pub fn gpsdata(&self) -> [u32; 32] { unsafe { (*self.imgdata.0).other.gpsdata } }
    ///Parsed GPS-data: longitude/latitude/altitude and time stamp.
    pub fn parsed_gps(&self) -> ImgOtherGpsInfo { ImgOtherGpsInfo::new(self.imgdata.clone()) }
    ///Image description.
    pub fn desc(&self) -> &CStr { unsafe { CStr::from_ptr((*self.imgdata.0).other.desc.as_ptr()) } }
    ///Author of image.
    pub fn artist(&self) -> &CStr {
        unsafe { CStr::from_ptr((*self.imgdata.0).other.artist.as_ptr()) }
    }
}
