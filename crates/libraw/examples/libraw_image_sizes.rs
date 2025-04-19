fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    let libraw = libraw::Libraw::new(None);
    let sizes = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_image_sizes()?;

    println!("{:?}", sizes);

    assert_eq!(sizes.raw_height(), 3516);
    assert_eq!(sizes.raw_width(), 5360);
    assert_eq!(sizes.height(), 3464);
    assert_eq!(sizes.width(), 5202);
    assert_eq!(sizes.top_margin(), 52);
    assert_eq!(sizes.left_margin(), 158);
    assert_eq!(sizes.iheight(), 3464);
    assert_eq!(sizes.iwidth(), 5202);
    assert_eq!(sizes.raw_pitch(), 10720);
    assert_eq!(sizes.pixel_aspect(), 1.0);
    assert_eq!(sizes.flip(), libraw::libraw::LibrawFlip::None);

    Ok(())
}
