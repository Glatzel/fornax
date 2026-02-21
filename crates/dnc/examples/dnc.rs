#[cfg(target_os = "linux")]
fn main() -> mischief::Result<()> { Ok(()) }
#[cfg(not(target_os = "linux"))]
fn main() -> mischief::Result<()> {
    fornax_devtool::example_setup();

    default_path()?;
    custom_path()?;
    match std::fs::remove_file(fornax_devtool::raw_file().with_extension("dng")) {
        Ok(()) => println!("dng file removed successfully."),
        Err(e) => eprintln!("Failed to remove file: {e}"),
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn default_path() -> mischief::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        overwrite: true,
        ..Default::default()
    });

    let dng_file = dnc.convert(&fornax_devtool::raw_file())?;
    assert!(dng_file.is_file());
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn custom_path() -> mischief::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        directory: Some(fornax_devtool::output_dir()),
        filename: Some("dng-converter.dng".to_string()),
        overwrite: true,
        ..Default::default()
    });
    let dng_file = dnc.convert(&fornax_devtool::raw_file())?;
    assert!(dng_file.is_file());
    Ok(())
}
#[test]
fn test() -> mischief::Result<()> { main() }
