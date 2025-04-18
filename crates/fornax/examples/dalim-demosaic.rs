use fornax::Fornax;
use fornax_dalim::{Dalim, DalimParams};
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();
    linear()?;
    Ok(())
}
fn linear() -> miette::Result<()> {
    let dalim = Dalim::<u16>::new(DalimParams {
        demosaicer: fornax_dalim::Demosaicer::Linear,
    });
    let mut manager = Fornax::new(libraw::Libraw::new(None), dalim);
    let img = manager.decode_file(&utils::raw_file())?.post_process()?;
    img.save(utils::output_dir().join("dalim-demosaic-linear.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving image.");
    Ok(())
}
