use std::ffi::CString;

use crate::utils;
use libraw_sys as sys;
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Clone, Debug)]
pub struct IParams {
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
    cdesc: String,
    xmplen: u32,
    xmpdata: String,
}
impl IParams {
    pub(crate) fn new(imgdata: *mut sys::libraw_data_t) -> miette::Result<Self> {
        if unsafe { (*imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        let imgdata = unsafe { *imgdata };
        Ok(Self {
            make: utils::mnt_to_string(&imgdata.idata.make),
            model: utils::mnt_to_string(&imgdata.idata.model),
            normalized_make: utils::mnt_to_string(&imgdata.idata.normalized_make),
            normalized_model: utils::mnt_to_string(&imgdata.idata.normalized_model),
            maker_index: imgdata.idata.maker_index,
            software: utils::mnt_to_string(&imgdata.idata.software),
            raw_count: imgdata.idata.raw_count,
            is_foveon: imgdata.idata.is_foveon != 0,
            dng_version: imgdata.idata.dng_version,
            colors: imgdata.idata.colors,
            filters: imgdata.idata.filters,
            xtrans: imgdata.idata.xtrans,
            xtrans_abs: imgdata.idata.xtrans_abs,
            cdesc: utils::mnt_to_string(&imgdata.idata.cdesc),
            xmplen: imgdata.idata.xmplen,
            xmpdata: unsafe {
                CString::from_raw(imgdata.idata.xmpdata)
                    .to_string_lossy()
                    .to_string()
            },
        })
    }
    pub fn make(&self) -> String {
        self.make.clone()
    }
    pub fn model(&self) -> String {
        self.model.clone()
    }
    pub fn normalized_make(&self) -> String {
        self.normalized_make.clone()
    }
    pub fn normalized_model(&self) -> String {
        self.normalized_model.clone()
    }
    pub fn maker_index(&self) -> u32 {
        self.maker_index
    }
    pub fn software(&self) -> String {
        self.software.clone()
    }
    pub fn raw_count(&self) -> u32 {
        self.raw_count
    }
    pub fn is_foveon(&self) -> bool {
        self.is_foveon
    }
    pub fn dng_version(&self) -> u32 {
        self.dng_version
    }
    pub fn colors(&self) -> i32 {
        self.colors
    }
    pub fn filters(&self) -> u32 {
        self.filters
    }
    pub fn xtrans(&self) -> [[i8; 6]; 6] {
        self.xtrans
    }
    pub fn xtrans_abs(&self) -> [[i8; 6]; 6] {
        self.xtrans_abs
    }
    pub fn cdesc(&self) -> String {
        self.cdesc.clone()
    }
    pub fn xmplen(&self) -> u32 {
        self.xmplen
    }
    pub fn xmpdata(&self) -> String {
        self.xmpdata.clone()
    }
}
