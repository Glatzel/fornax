use std::io::Read;

use fornax::Fornax;
use libraw::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawParams,
    DCRawUserFlip, DCRawUserQual,
};
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    default_settings()?;
    cg()?;
    custom()?;
    Ok(())
}
fn default_settings() -> miette::Result<()> {
    let dcraw_params = DCRawParams {
        user_qual: Some(libraw::DCRawUserQual::Linear),
        ..Default::default()
    };
    let libraw = libraw::Libraw::new(Some(dcraw_params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u8> = Fornax::new(&libraw, &libraw);
    let img = manager
        .decode_file(&fornax_devtool::raw_file())?
        .post_process()?;
    img.save(fornax_devtool::output_dir().join("fornax-libraw-libraw-default.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving :fornax-libraw-libraw-default.tiff");
    Ok(())
}
fn cg() -> miette::Result<()> {
    let mut file = std::fs::File::open(fornax_devtool::raw_file()).into_diagnostic()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).into_diagnostic()?;
    let params = libraw::DCRawParams::preset_cg();
    let libraw = libraw::Libraw::new(Some(params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    let img = manager.decode_buffer(&buffer)?.post_process()?;
    img.save(fornax_devtool::output_dir().join("fornax-libraw-libraw-cg.tiff"))
        .into_diagnostic()?;
    clerk::info!("save img to: fornax-libraw-libraw-.tiff");
    Ok(())
}
fn custom() -> miette::Result<()> {
    let mut file = std::fs::File::open(fornax_devtool::raw_file()).into_diagnostic()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).into_diagnostic()?;
    let params = libraw::DCRawParams {
        greybox: None,
        cropbox: None,
        aber: None,
        gamm: Some([1.0, 1.0]),
        user_mul: Some([0.9, 0.8, 0.7, 0.6]),
        bright: Some(0.9),
        threshold: Some(0.1),
        half_size: Some(true),
        four_color_rgb: Some(false),
        highlight: Some(DCRawHighlightMode::Reconstruct5),
        use_auto_wb: Some(true),
        use_camera_wb: Some(true),
        use_camera_matrix: None,
        output_color: Some(DCRawOutputColor::ACES),
        output_profile: None,
        camera_profile: None,
        bad_pixels: None,
        dark_frame: None,
        output_bps: Some(DCRawOutputBps::_16bit),
        output_tiff: None,
        user_flip: Some(DCRawUserFlip::CW90),
        user_qual: Some(DCRawUserQual::Linear),
        user_black: Some(5),
        user_cblack: Some([1, 2, 3, 4]),
        user_sat: Some(3),
        med_passes: Some(3),
        no_auto_bright: Some(true),
        auto_bright_thr: Some(5.0),
        adjust_maximum_thr: Some(0.6),
        use_fuji_rotate: None,
        green_matching: Some(true),
        dcb_iterations: Some(3),
        dcb_enhance_fl: Some(3),
        fbdd_noiserd: Some(DCRawFbddNoiserd::Off),
        exp_correc: Some(3),
        exp_shift: Some(3.0),
        exp_preser: Some(3.0),
        use_rawspeed: Some(true),
        no_auto_scale: Some(true),
        no_interpolation: Some(true),
    };
    let libraw = libraw::Libraw::new(Some(params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    let img = manager.decode_buffer(&buffer)?.post_process()?;
    img.save(fornax_devtool::output_dir().join("fornax-libraw-libraw-custom.tiff"))
        .into_diagnostic()?;
    clerk::info!("save img to: custom.tiff");
    Ok(())
}
