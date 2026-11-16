//! Large zero-initialized scalar buffers with an optional Linux huge-page hint.

mod large_alloc;

pub use large_alloc::{AllocError, zeroed_samples};
