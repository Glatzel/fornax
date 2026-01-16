use chrono::{DateTime, Utc};
use envoy::CStrToString;

use crate::LibrawError;

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ImgOtherGpsInfo {
    latitude: [f32; 3usize],
    longitude: [f32; 3usize],
    gpstimestamp: [f32; 3usize],
    altitude: f32,
    altref: String,
    latref: String,
    longref: String,
    gpsstatus: String,
    gpsparsed: String,
}
impl ImgOtherGpsInfo {
    pub(crate) fn new(info: libraw_sys::libraw_gps_info_t) -> Self {
        Self {
            latitude: info.latitude,
            longitude: info.longitude,
            gpstimestamp: info.gpstimestamp,
            altitude: info.altitude,
            altref: info.altref.to_string(),
            latref: info.latref.to_string(),
            longref: info.longref.to_string(),
            gpsstatus: info.gpsstatus.to_string(),
            gpsparsed: info.gpsparsed.to_string(),
        }
    }
    pub fn latitude(&self) -> [f32; 3usize] { self.latitude }
    pub fn longitude(&self) -> [f32; 3usize] { self.longitude }
    pub fn gpstimestamp(&self) -> [f32; 3usize] { self.gpstimestamp }
    pub fn altitude(&self) -> f32 { self.altitude }
    pub fn altref(&self) -> &str { &self.altref }
    pub fn latref(&self) -> &str { &self.latref }
    pub fn longref(&self) -> &str { &self.longref }
    pub fn gpsstatus(&self) -> &str { &self.gpsstatus }
    pub fn gpsparsed(&self) -> &str { &self.gpsparsed }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ImgOther {
    iso_speed: f32,
    shutter: f32,
    aperture: f32,
    focal_len: f32,
    timestamp: DateTime<Utc>,
    shot_order: u32,
    gpsdata: [u32; 32],
    parsed_gps: ImgOtherGpsInfo,
    desc: String,
    artist: String,
}
impl ImgOther {
    pub(crate) fn new(imgdata: *mut libraw_sys::libraw_data_t) -> Result<Self, LibrawError> {
        let imgdata = unsafe { *imgdata };
        let parsed_gps = ImgOtherGpsInfo::new(imgdata.other.parsed_gps);
        Ok(Self {
            // make: utils::mnt_to_string(&imgdata.idata.make),
            iso_speed: imgdata.other.iso_speed,
            shutter: imgdata.other.shutter,
            aperture: imgdata.other.aperture,
            focal_len: imgdata.other.focal_len,
            timestamp: DateTime::from_timestamp(imgdata.other.timestamp, 0).unwrap(),
            shot_order: imgdata.other.shot_order,
            gpsdata: imgdata.other.gpsdata,
            parsed_gps,
            desc: CStrToString::to_string(imgdata.other.desc.as_slice()).unwrap_or_default(),
            artist: CStrToString::to_string(imgdata.other.artist.as_slice()).unwrap_or_default(),
        })
    }
    ///ISO sensitivity.
    pub fn iso_speed(&self) -> f32 { self.iso_speed }
    ///Shutter speed.
    pub fn shutter(&self) -> f32 { self.shutter }
    ///Aperture.
    pub fn aperture(&self) -> f32 { self.aperture }
    ///Focal length.
    pub fn focal_len(&self) -> f32 { self.focal_len }
    ///Date of shooting.
    pub fn timestamp(&self) -> DateTime<Utc> { self.timestamp }
    ///Serial number of image.
    pub fn shot_order(&self) -> u32 { self.shot_order }
    ///GPS data (unparsed block, to write to output as is).
    pub fn gpsdata(&self) -> [u32; 32] { self.gpsdata }
    ///Parsed GPS-data: longitude/latitude/altitude and time stamp.
    pub fn parsed_gps(&self) -> ImgOtherGpsInfo { self.parsed_gps.clone() }
    ///Image description.
    pub fn desc(&self) -> String { self.desc.clone() }
    ///Author of image.
    pub fn artist(&self) -> String { self.artist.clone() }
}
