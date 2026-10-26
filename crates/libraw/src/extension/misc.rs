use fornax_core::{BayerPattern, Primitive};
use fornax_decoder_protocol::{DecodedRawActive, DecodedRawFull};

use super::super::{Libraw, LibrawError};
use super::error_handling::{check_run, custom_error};

// region:Custom API
impl Libraw {
    pub fn bayer_pattern(&self) -> Result<BayerPattern, LibrawError> {
        let filters = self.get_iparams()?.filters();
        match filters {
            0 => {
                custom_error!("non-mosaic RAW is unsupported");
            }
            1 => { 
                custom_error!("16x16 CFA is unsupported");
            }
            9 => {
                custom_error!("X-Trans CFA is unsupported");
            }
            2..=8 | 10..=999 => {
                custom_error!(format!("reserved LibRaw CFA descriptor {filters}"));
            }
            _ => {}
        }

        // LibRaw COLOR(row, column) uses two bits per CFA color.
        let color = |row: u32, col: u32| -> u32 { (filters >> (((row << 1) & 14) + col) * 2) & 3 };

        match (color(0, 0), color(0, 1), color(1, 0), color(1, 1)) {
            (0, 1, 3, 2) => Ok(BayerPattern::RGGB),
            (2, 3, 1, 0) => Ok(BayerPattern::BGGR),
            (1, 0, 2, 3) => Ok(BayerPattern::GRBG),
            (3, 2, 0, 1) => Ok(BayerPattern::GBRG),
            _ => custom_error!("unsupported LibRaw CFA pattern {filters:#010x}"),
        }
    }
    pub fn get_decoded_raw_active<T>(&self) -> Result<DecodedRawActive<T>, LibrawError>
    where
        T: Primitive,
    {
        todo!()
    }
    pub fn get_decoded_raw_full<T>(&self) -> Result<DecodedRawFull<T>, LibrawError>
    where
        T: Primitive,
    {
        todo!()
    }
    pub fn get_params(&self) -> Result<libraw_sys::libraw_output_params_t, LibrawError> {
        Ok(unsafe { (*self.imgdata_ptr()).params })
    }
}
