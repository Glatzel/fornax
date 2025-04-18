use fornax::Fornax;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let params = libraw::dcraw::DCRawParams::preset_cg();
    let libraw = libraw::Libraw::new(Some(params));
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    let img = manager.decode_file(&utils::raw_file())?.post_process()?;
    img.save(utils::output_dir().join("process-cg.tiff"))
        .into_diagnostic()?;
    clerk::info!("save img to: process-cg.tiff");
    Ok(())
}
