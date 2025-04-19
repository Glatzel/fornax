use fornax::Fornax;
use fornax_dalim::{Dalim, DalimParams};
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    linear()?;
    Ok(())
}
fn linear() -> miette::Result<()> {
    let dalim = Dalim::<u16>::new(DalimParams {
        demosaicer: fornax_dalim::Demosaicer::Linear,
    });
    let manager = Fornax::new(libraw::Libraw::new(None), dalim);
    let img = manager
        .decode_file(&fornax_devtool::raw_file())?
        .post_process()?;
    img.save(fornax_devtool::output_dir().join("dalim-demosaic-linear.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving image.");
    Ok(())
}
