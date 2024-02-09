#[cfg(feature = "rkyv")]
mod custom;
#[cfg(not(feature = "rkyv"))]
mod json;

#[cfg(feature = "rkyv")]
pub use custom::*;

#[cfg(not(feature = "rkyv"))]
pub use json::*;
