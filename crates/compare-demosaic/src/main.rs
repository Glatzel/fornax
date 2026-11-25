use clap::Parser;
use clerk::tracing_subscriber::layer::SubscriberExt;
use clerk::tracing_subscriber::util::SubscriberInitExt;
use clerk::tracing_subscriber::{EnvFilter, Layer, Registry};
use compare_demosaic::{CliArgs, run};

fn main() -> mischief::Result<()> {
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
        println!(
            "{} {} -> {} ({}x{})",
            outcome.method.as_str(),
            outcome.pattern,
            outcome.output_path.display(),
            outcome.width,
            outcome.height
        );
    }
    Ok(())
}
