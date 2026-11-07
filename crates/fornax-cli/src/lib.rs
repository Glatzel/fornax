//! Batch RAW conversion for Fornax.

mod config;
mod convert;
mod discover;

pub use config::{
    CliArgs, ConfigError, DngCompressionChoice, OutputFormat, ResolvedConfig,
    load_config, resolve_config,
};
pub use convert::{ConversionOutcome, run};
pub use discover::{InputJob, plan_inputs};
