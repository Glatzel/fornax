fn main() -> mischief::Result<()> {
    let path = compare_demosaic::results::generate()?.write_default()?;
    println!("wrote {}", path.display());
    Ok(())
}
