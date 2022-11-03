#![forbid(unsafe_code)]

pub mod util;

#[cfg(feature = "derive")]
pub use dfdi_macros::Service;

pub use dfdi_core::*;
