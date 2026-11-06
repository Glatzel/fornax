use clap::Parser;
use fornax_cli::{CliArgs, run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    for outcome in run(&args)? {
        println!("{outcome}");
    }
    Ok(())
}
