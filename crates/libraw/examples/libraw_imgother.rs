mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let libraw = libraw::Libraw::new(None);
    let imgother = libraw
        .open_file(&utils::raw_file())?
        .unpack()?
        .get_imgother()?;
    println!("{:?}", imgother);

    assert_eq!(imgother.iso_speed(), 100.0);
    assert_eq!(imgother.aperture(), 5.6);
    assert_eq!(imgother.focal_len(), 30.0);

    Ok(())
}
