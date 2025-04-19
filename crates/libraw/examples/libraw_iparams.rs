mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let libraw = libraw::Libraw::new(None);
    let iparams = libraw
        .open_file(&utils::raw_file())?
        .unpack()?
        .get_iparams()?;

    println!("{:?}", iparams);

    assert_eq!(iparams.make(), "Canon");
    assert_eq!(iparams.model(), "EOS 7D");
    assert_eq!(iparams.normalized_make(), "Canon");
    assert_eq!(iparams.normalized_model(), "EOS 7D");
    assert_eq!(iparams.colors(), 3);

    Ok(())
}
