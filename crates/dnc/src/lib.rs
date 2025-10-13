mod error;
mod params;
pub use error::DncError;
pub use params::DncParams;
#[cfg(not(target_os = "linux"))]
mod wrapper;
#[cfg(not(target_os = "linux"))]
pub use wrapper::*;
