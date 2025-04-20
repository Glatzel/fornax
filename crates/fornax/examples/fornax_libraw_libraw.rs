use std::io::Read;

use fornax::Fornax;
use libraw::dcraw::DCRawParams;
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    default_settings()?;
    cg()?;
    Ok(())
}
fn default_settings() -> miette::Result<()> {
    let dcraw_params = DCRawParams {
        user_qual: Some(libraw::dcraw::DCRawUserQual::Linear),
        ..Default::default()
    };
    let libraw = libraw::Libraw::new(Some(dcraw_params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    let img = manager
        .decode_file(&fornax_devtool::raw_file())?
        .post_process()?;
    img.save(fornax_devtool::output_dir().join("process.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
fn cg() -> miette::Result<()> {
    let mut file = std::fs::File::open(fornax_devtool::raw_file()).into_diagnostic()?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).into_diagnostic()?;
    let params = libraw::dcraw::DCRawParams::preset_cg();
    let libraw = libraw::Libraw::new(Some(params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    let img = manager.decode_buffer(&buffer)?.post_process()?;
    img.save(fornax_devtool::output_dir().join("process-cg.tiff"))
        .into_diagnostic()?;
    clerk::info!("save img to: process-cg.tiff");
    Ok(())
}
