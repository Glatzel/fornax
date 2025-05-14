use fornax_core::BayerChannel;

use crate::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawUserQual, Libraw,
    LibrawIParams, LibrawImgOther, check_raw_alloc,
};

// region:Parameters setters/getters
impl Libraw {
    pub fn get_raw_height(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_height(self.imgdata) })
    }
    pub fn get_raw_width(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_raw_width(self.imgdata) })
    }
    pub fn get_iheight(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iheight(self.imgdata) })
    }
    pub fn get_iwidth(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_iwidth(self.imgdata) })
    }
    pub fn get_cam_mul(&self, index: BayerChannel) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_cam_mul(self.imgdata, u8::from(index) as i32) })
    }
    pub fn get_pre_mul(&self, index: BayerChannel) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_pre_mul(self.imgdata, u8::from(index) as i32) })
    }
    pub fn get_rgb_cam(&self, index1: i32, index2: i32) -> miette::Result<f32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_rgb_cam(self.imgdata, index1, index2) })
    }
    pub fn get_iparams(&self) -> miette::Result<LibrawIParams> {
        check_raw_alloc!(self.imgdata);
        LibrawIParams::new(self.imgdata)
    }
    pub fn get_lensinfo(&self) { unimplemented!() }
    pub fn get_imgother(&self) -> miette::Result<LibrawImgOther> {
        check_raw_alloc!(self.imgdata);
        LibrawImgOther::new(self.imgdata)
    }
    pub fn get_color_maximum(&self) -> miette::Result<i32> {
        check_raw_alloc!(self.imgdata);
        Ok(unsafe { libraw_sys::libraw_get_color_maximum(self.imgdata) })
    }
    pub fn set_user_mul(&self, index: i32, val: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_user_mul(self.imgdata, index, val) };
        self
    }
    pub fn set_demosaic(&self, value: DCRawUserQual) -> &Self {
        unsafe { libraw_sys::libraw_set_demosaic(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_adjust_maximum_thr(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_adjust_maximum_thr(self.imgdata, value) };
        self
    }
    pub fn set_output_color(&self, value: DCRawOutputColor) -> &Self {
        unsafe { libraw_sys::libraw_set_output_color(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_output_bps(&self, value: DCRawOutputBps) -> &Self {
        unsafe { libraw_sys::libraw_set_output_bps(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_gamma(&self, index: i32, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_gamma(self.imgdata, index, value) };
        self
    }
    pub fn set_no_auto_bright(&self, value: bool) -> &Self {
        unsafe { libraw_sys::libraw_set_no_auto_bright(self.imgdata, value as i32) };
        self
    }
    pub fn set_bright(&self, value: f32) -> &Self {
        unsafe { libraw_sys::libraw_set_bright(self.imgdata, value) };
        self
    }
    pub fn set_highlight(&self, value: DCRawHighlightMode) -> &Self {
        unsafe { libraw_sys::libraw_set_highlight(self.imgdata, i32::from(value)) };
        self
    }
    pub fn set_fbdd_noiserd(&self, value: DCRawFbddNoiserd) -> &Self {
        unsafe { libraw_sys::libraw_set_fbdd_noiserd(self.imgdata, i32::from(value)) };
        self
    }
}

#[cfg(test)]
mod test {
    use float_cmp::assert_approx_eq;

    use super::*;

    // region:Parameters setters/getters
    #[test]
    fn test_get_raw_height() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_height()?;
        assert_eq!(3516, value);
        Ok(())
    }
    #[test]
    fn test_get_raw_width() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_raw_width()?;
        assert_eq!(5360, value);
        Ok(())
    }
    #[test]
    fn test_get_iheight() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iheight()?;
        assert_eq!(3464, value);
        Ok(())
    }
    #[test]
    fn test_get_iwidth() -> miette::Result<()> {
        let libraw = Libraw::default();
        let value = libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .get_iwidth()?;
        assert_eq!(5202, value);
        Ok(())
    }
    #[test]
    fn test_get_cam_mul() -> miette::Result<()> {
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
    fn test_get_pre_mul() -> miette::Result<()> {
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
    fn test_get_rgb_cam() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_rgb_cam(1, 2)?;
        assert_approx_eq!(f32, -0.5123857, value);
        Ok(())
    }
    #[test]
    fn test_get_color_maximum() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw.open_file(&fornax_devtool::raw_file())?.unpack()?;
        let value = libraw.get_color_maximum()?;
        assert_eq!(13584, value);
        Ok(())
    }
    #[test]
    fn test_set_user_mul() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_user_mul(1, 2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.user_mul[1]);
        Ok(())
    }
    #[test]
    fn test_set_demosaic() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_demosaic(DCRawUserQual::Linear);
        assert_eq!(
            i32::from(DCRawUserQual::Linear),
            libraw.get_params()?.user_qual
        );
        Ok(())
    }
    #[test]
    fn test_set_adjust_maximum_thr() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_adjust_maximum_thr(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.adjust_maximum_thr);
        Ok(())
    }
    #[test]
    fn test_set_output_color() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_color(DCRawOutputColor::ACES);
        assert_eq!(
            i32::from(DCRawOutputColor::ACES),
            libraw.get_params()?.output_color
        );
        Ok(())
    }
    #[test]
    fn set_output_bps() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_output_bps(DCRawOutputBps::_16bit);
        assert_eq!(
            i32::from(DCRawOutputBps::_16bit),
            libraw.get_params()?.output_bps
        );
        Ok(())
    }
    #[test]
    fn test_set_gamma() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_gamma(1, 2.0);
        assert_approx_eq!(f64, 2.0, libraw.get_params()?.gamm[1]);
        Ok(())
    }
    #[test]
    fn test_set_no_auto_bright() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_no_auto_bright(true);
        assert!(libraw.get_params()?.no_auto_bright != 0);
        Ok(())
    }
    #[test]
    fn test_set_bright() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_bright(2.0);
        assert_approx_eq!(f32, 2.0, libraw.get_params()?.bright);
        Ok(())
    }
    #[test]
    fn test_set_highlight() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_highlight(DCRawHighlightMode::Reconstruct4);
        assert_eq!(
            i32::from(DCRawHighlightMode::Reconstruct4),
            libraw.get_params()?.highlight
        );
        Ok(())
    }
    #[test]
    fn set_fbdd_noiserd() -> miette::Result<()> {
        let libraw = Libraw::default();
        libraw
            .open_file(&fornax_devtool::raw_file())?
            .unpack()?
            .set_fbdd_noiserd(DCRawFbddNoiserd::Off);
        assert_eq!(
            i32::from(DCRawFbddNoiserd::Off),
            libraw.get_params()?.fbdd_noiserd
        );
        Ok(())
    }
}
