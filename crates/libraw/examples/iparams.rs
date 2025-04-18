mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let manager = Fornax::new(libraw::Libraw::new(None), fornax::NullPostProcessor {});
    manager.decode_file(&utils::raw_file())?;

    let iparams = manager.decoder.idata()?;
    println!("{:?}", iparams);
    Ok(())
}
