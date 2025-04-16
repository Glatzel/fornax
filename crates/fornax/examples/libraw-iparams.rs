use std::path::PathBuf;

use fornax::Fornax;
mod utils;
fn main() -> miette::Result<()> {
    utils::init_log();
    utils::creat_output_dir();
    let mut manager = Fornax::new(libraw::Libraw::new(None), fornax::NullPostProcessor {});
    manager.decode_file(&utils::raw_file())?;

    let iparams = manager.decoder.iparams()?;
    println!("{:?}", iparams);
    Ok(())
}
