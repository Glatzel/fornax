fn main() -> mischief::Result<()> {
    fornax_devtool::example_setup();

    let libraw = libraw::Libraw::default();
    let iparams = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_iparams()?;

    println!("{iparams:?}");

    assert_eq!(iparams.make().to_str()?, "Canon");
    assert_eq!(iparams.model().to_str()?, "EOS 7D");
    assert_eq!(iparams.normalized_make().to_str()?, "Canon");
    assert_eq!(iparams.normalized_model().to_str()?, "EOS 7D");
    assert_eq!(iparams.colors(), 3);

    Ok(())
}
