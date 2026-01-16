use fornax_core::BayerChannel;

use crate::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawUserQual, IParams,
    ImgOther, Libraw, LibrawError, check_raw_alloc,
};

// region:Parameters setters/getters
impl Libraw {
    pub fn get_raw_height(&self) -> Result<i32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_height(self.imgdata.0) })
    }
    pub fn get_raw_width(&self) -> Result<i32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_width(self.imgdata.0) })
    }
    pub fn get_iheight(&self) -> Result<i32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iheight(self.imgdata.0) })
    }
    pub fn get_iwidth(&self) -> Result<i32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iwidth(self.imgdata.0) })
    }
    pub fn get_cam_mul(&self, index: BayerChannel) -> Result<f32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_cam_mul(self.imgdata.0, u8::from(index) as i32) })
    }
    pub fn get_pre_mul(&self, index: BayerChannel) -> Result<f32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_pre_mul(self.imgdata.0, u8::from(index) as i32) })
    }
    pub fn get_rgb_cam(&self, index1: i32, index2: i32) -> Result<f32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_rgb_cam(self.imgdata.0, index1, index2) })
    }
    pub fn get_iparams(&self) -> Result<IParams, LibrawError> {
        check_raw_alloc!(self.imgdata);
        IParams::new(self.imgdata.clone())
    }
    pub fn get_lensinfo(&self) { todo!() }
    pub fn get_imgother(&self) -> Result<ImgOther, LibrawError> {
        check_raw_alloc!(self.imgdata);
        ImgOther::new(self.imgdata.clone())
    }
    pub fn get_color_maximum(&self) -> Result<i32, LibrawError> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_color_maximum(self.imgdata.0) })
    }
    pub fn set_user_mul(&self, index: i32, val: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_user_mul(self.imgdata.0, index, val) };
        self
    }
    pub fn set_demosaic(&self, value: DCRawUserQual) -> &Self {
        unsafe { libraw_sys::libraw_set_demosaic(self.imgdata.0, value as i32) };
        self
    }
    pub fn set_adjust_maximum_thr(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_adjust_maximum_thr(self.imgdata.0, value) };
        self
    }
    pub fn set_output_color(&self, value: DCRawOutputColor) -> &Self {
        unsafe { libraw_sys::libraw_set_output_color(self.imgdata.0, value as i32) };
        self
    }
    pub fn set_output_bps(&self, value: DCRawOutputBps) -> &Self {
        unsafe { libraw_sys::libraw_set_output_bps(self.imgdata.0, value as i32) };
        self
    }
    pub fn set_gamma(&self, index: i32, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_gamma(self.imgdata.0, index, value) };
        self
    }
    pub fn set_no_auto_bright(&self, value: bool) -> &Self {
        unsafe { libraw_sys::libraw_set_no_auto_bright(self.imgdata.0, value as i32) };
        self
    }
    pub fn set_bright(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_bright(self.imgdata.0, value) };
        self
    }
    pub fn set_highlight(&self, value: DCRawHighlightMode) -> &Self {
        unsafe { libraw_sys::libraw_set_highlight(self.imgdata.0, value as i32) };
        self
    }
    pub fn set_fbdd_noiserd(&self, value: DCRawFbddNoiserd) -> &Self {
        unsafe { libraw_sys::libraw_set_fbdd_noiserd(self.imgdata.0, value as i32) };
        self
    }
}

#[cfg(test)]
mod test {
    use float_cmp::assert_approx_eq;

    use super::*;

    // region:Parameters setters/getters
    #[test]
    fn test_get_raw_height() -> mischief::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_height()?;
        assert_eq!(3516, value);
        Ok(())
    }
    #[test]
    fn test_get_raw_width() -> mischief::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_width()?;
        assert_eq!(5360, value);
        Ok(())
    }
    #[test]
    fn test_get_iheight() -> mischief::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iheight()?;
        assert_eq!(3464, value);
        Ok(())
    }
    #[test]
    fn test_get_iwidth() -> mischief::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iwidth()?;
        assert_eq!(5202, value);
        Ok(())
    }
    #[test]
    fn test_get_cam_mul() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;

        let value = libraw.get_cam_mul(BayerChannel::R)?;
        assert_eq!(2127.0, value);
        let value = libraw.get_cam_mul(BayerChannel::G)?;
        assert_eq!(1024.0, value);
        let value = libraw.get_cam_mul(BayerChannel::B)?;
        assert_eq!(1584.0, value);
        let value = libraw.get_cam_mul(BayerChannel::G2)?;
        assert_eq!(1024.0, value);
        Ok(())
    }
    #[test]
    fn test_get_pre_mul() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;

        let value = libraw.get_pre_mul(BayerChannel::R)?;
        assert_approx_eq!(f32, 2.1848476, value);
        let value = libraw.get_pre_mul(BayerChannel::G)?;
        assert_approx_eq!(f32, 0.9358199, value);
        let value = libraw.get_pre_mul(BayerChannel::B)?;
        assert_approx_eq!(f32, 1.2567647, value);
        let value = libraw.get_pre_mul(BayerChannel::G2)?;
        assert_approx_eq!(f32, 0.0, value);
        Ok(())
    }
    #[test]
    fn test_get_rgb_cam() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_rgb_cam(1, 2)?;
        assert_approx_eq!(f32, -0.5123857, value);
        Ok(())
    }
    #[test]
    fn test_get_color_maximum() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_color_maximum()?;
        assert_eq!(13584, value);
        Ok(())
    }
    #[test]
    fn test_set_user_mul() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_user_mul(1, 2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.user_mul[1]);
        Ok(())
    }
    #[test]
    fn test_set_demosaic() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_demosaic(DCRawUserQual::Linear);
        assert_eq!(DCRawUserQual::Linear as i32, libraw.get_params()?.user_qual);
        Ok(())
    }
    #[test]
    fn test_set_adjust_maximum_thr() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_adjust_maximum_thr(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.adjust_maximum_thr);
        Ok(())
    }
    #[test]
    fn test_set_output_color() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_color(DCRawOutputColor::ACES);
        assert_eq!(
            DCRawOutputColor::ACES as i32,
            libraw.get_params()?.output_color
        );
        Ok(())
    }
    #[test]
    fn set_output_bps() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_bps(DCRawOutputBps::_16bit);
        assert_eq!(
            DCRawOutputBps::_16bit as i32,
            libraw.get_params()?.output_bps
        );
        Ok(())
    }
    #[test]
    fn test_set_gamma() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_gamma(1, 2.0);
        assert_approx_eq!(f64, 2.0, libraw.get_params()?.gamm[1]);
        Ok(())
    }
    #[test]
    fn test_set_no_auto_bright() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_no_auto_bright(true);
        assert!(libraw.get_params()?.no_auto_bright != 0);
        Ok(())
    }
    #[test]
    fn test_set_bright() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_bright(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.bright);
        Ok(())
    }
    #[test]
    fn test_set_highlight() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_highlight(DCRawHighlightMode::Reconstruct4);
        assert_eq!(
            DCRawHighlightMode::Reconstruct4 as i32,
            libraw.get_params()?.highlight
        );
        Ok(())
    }
    #[test]
    fn set_fbdd_noiserd() -> mischief::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_fbdd_noiserd(DCRawFbddNoiserd::Off);
        assert_eq!(
            DCRawFbddNoiserd::Off as i32,
            libraw.get_params()?.fbdd_noiserd
        );
        Ok(())
    }
}
