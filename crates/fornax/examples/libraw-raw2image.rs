use std::path::PathBuf;

use fornax::Fornax;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let mut manager = Fornax::new(libraw::Libraw::new(None), fornax::NullPostProcessor {});
    manager.decode_file(&utils::raw_file())?;

    let img = manager.decoder.raw2image(true)?;
    clerk::info!("Done building raw image.");

    img.save(utils::output_dir().join("raw2image.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
