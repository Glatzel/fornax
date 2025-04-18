use fornax::Fornax;
use libraw::dcraw::DCRawParams;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let dcraw_params = DCRawParams {
        user_qual: Some(libraw::dcraw::DCRawUserQual::Linear),
        ..Default::default()
    };
    let libraw = libraw::Libraw::new(Some(dcraw_params));
    let mut manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> =
        Fornax::new(&libraw, &libraw);
    let img = manager.decode_file(&utils::raw_file())?.post_process()?;
    img.save(utils::output_dir().join("process.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
