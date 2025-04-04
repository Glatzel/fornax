use chrono::{DateTime, Utc};

use crate::utils;
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone)]
pub struct GpsInfo {
    latitude: [f32; 3usize],
    longitude: [f32; 3usize],
    gpstimestamp: [f32; 3usize],
    altitude: f32,
    altref: char,
    latref: char,
    longref: char,
    gpsstatus: char,
    gpsparsed: char,
}
impl GpsInfo {
    pub(crate) fn new(info: libraw_sys::libraw_gps_info_t) -> Self {
        Self {
            latitude: info.latitude,
            longitude: info.longitude,
            gpstimestamp: info.gpstimestamp,
            altitude: info.altitude,
            altref: char::from(info.altref as u8),
            latref: char::from(info.latref as u8),
            longref: char::from(info.longref as u8),
            gpsstatus: char::from(info.gpsstatus as u8),
            gpsparsed: char::from(info.gpsparsed as u8),
        }
    }
    pub fn latitude(&self) -> [f32; 3usize] {
        self.latitude
    }
    pub fn longitude(&self) -> [f32; 3usize] {
        self.longitude
    }
    pub fn gpstimestamp(&self) -> [f32; 3usize] {
        self.gpstimestamp
    }
    pub fn altitude(&self) -> f32 {
        self.altitude
    }
    pub fn altref(&self) -> char {
        self.altref
    }
    pub fn latref(&self) -> char {
        self.latref
    }
    pub fn longref(&self) -> char {
        self.longref
    }
    pub fn gpsstatus(&self) -> char {
        self.gpsstatus
    }
    pub fn gpsparsed(&self) -> char {
        self.gpsparsed
    }
}
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone)]
pub struct ImgOther {
    iso_speed: f32,
    shutter: f32,
    aperture: f32,
    focal_len: f32,
    timestamp: DateTime<Utc>,
    shot_order: u32,
    gpsdata: [u32; 32],
    parsed_gps: GpsInfo,
    desc: String,
    artist: String,
}
impl ImgOther {
    pub(crate) fn new(imgdata: *mut libraw_sys::libraw_data_t) -> miette::Result<Self> {
        let imgdata = unsafe { *imgdata };
        let parsed_gps = GpsInfo::new(imgdata.other.parsed_gps);
        Ok(Self {
            // make: utils::mnt_to_string(&imgdata.idata.make),
            iso_speed: imgdata.other.iso_speed,
            shutter: imgdata.other.shutter,
            aperture: imgdata.other.aperture,
            focal_len: imgdata.other.focal_len,
            timestamp: DateTime::from_timestamp(imgdata.other.timestamp, 0).unwrap(),
            shot_order: imgdata.other.shot_order,
            gpsdata: imgdata.other.gpsdata,
            parsed_gps: parsed_gps,
            desc: utils::mnt_to_string(&imgdata.other.desc),
            artist: utils::mnt_to_string(&imgdata.other.artist),
        })
    }
    pub fn iso_speed(&self) -> f32 {
        self.iso_speed
    }
    pub fn shutter(&self) -> f32 {
        self.shutter
    }
    pub fn aperture(&self) -> f32 {
        self.aperture
    }
    pub fn focal_len(&self) -> f32 {
        self.focal_len
    }
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
    pub fn shot_order(&self) -> u32 {
        self.shot_order
    }
    pub fn gpsdata(&self) -> [u32; 32] {
        self.gpsdata
    }
    pub fn parsed_gps(&self) -> GpsInfo {
        self.parsed_gps.clone()
    }
    pub fn desc(&self) -> String {
        self.desc.clone()
    }
    pub fn artist(&self) -> String {
        self.artist.clone()
    }
}
