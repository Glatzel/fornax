mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    default_path()?;
    custom_path()?;
    match std::fs::remove_file(utils::raw_file().with_extension("dng")) {
        Ok(()) => println!("dng file removed successfully."),
        Err(e) => eprintln!("Failed to remove file: {}", e),
    }
    Ok(())
}

fn default_path() -> miette::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        overwrite: true,
        ..Default::default()
    });

    let dng_file = dnc.convert(&utils::raw_file())?;
    assert!(dng_file.is_file());
    Ok(())
}

fn custom_path() -> miette::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        directory: Some(utils::output_dir()),
        filename: Some("dng-converter.dng".to_string()),
        overwrite: true,
        ..Default::default()
    });
    let dng_file = dnc.convert(&utils::raw_file())?;
    assert!(dng_file.is_file());
    Ok(())
}
