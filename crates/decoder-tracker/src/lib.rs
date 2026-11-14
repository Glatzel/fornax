//! Decoder corpus manifest, execution, merge, and regression comparison.

pub mod corpus_metadata;
pub mod csvio;
pub mod duckdb;
pub mod golden;
pub mod manifest;
pub mod model;
pub mod runner;
pub mod selection;
pub mod stages;

#[cfg(test)]
mod tests;
