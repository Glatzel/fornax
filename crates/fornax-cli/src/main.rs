use clap::Parser;
use clerk::tracing_subscriber::layer::SubscriberExt;
use clerk::tracing_subscriber::util::SubscriberInitExt;
use clerk::tracing_subscriber::{EnvFilter, Layer, Registry};
use fornax_cli::{CliArgs, run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    Registry::default()
        .with(
            clerk::terminal_layer(true).with_filter(
                EnvFilter::builder()
                    .with_default_directive(
                        format!("{}={}", env!("CARGO_PKG_NAME"), args.verbose.filter()).parse()?,
                    )
                    .from_env_lossy(),
            ),
        )
        .init();
    for outcome in run(&args)? {
        println!("{outcome}");
    }
    Ok(())
}
