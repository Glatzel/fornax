//! Large zero-initialized scalar buffers with an optional Linux huge-page hint.
#![deny(unsafe_code)]

#[allow(unsafe_code)]
mod large_alloc;

pub use large_alloc::{AllocError, zeroed_samples};
