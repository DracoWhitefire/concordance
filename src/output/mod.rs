//! Output types produced by the negotiation pipeline.

pub mod warning;

#[cfg(any(feature = "alloc", feature = "std"))]
pub mod config;
#[cfg(any(feature = "alloc", feature = "std"))]
pub mod rejection;
#[cfg(any(feature = "alloc", feature = "std"))]
pub mod trace;
