fn main() -> mischief::Result<()> {
    let path = compare_demosaic::readme::update_default_readme()?;
    println!("wrote {}", path.display());
    Ok(())
}
