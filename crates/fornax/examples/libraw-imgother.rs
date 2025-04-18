use fornax::Fornax;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let manager = Fornax::new(libraw::Libraw::new(None), fornax::NullPostProcessor {});
    manager.decode_file(&utils::raw_file())?;

    let other = manager.decoder.other()?;
    println!("{:?}", other);
    Ok(())
}
