#![forbid(unsafe_code)]
//! One source for contracts shared by independent agent mechanisms.
//! This crate performs no I/O, authentication, approval, or task transitions.

pub mod compilation;
pub mod identity;
pub mod json;
pub mod limits;
#[cfg(feature = "serde")]
pub mod model;
#[cfg(feature = "serde")]
pub mod provider;
pub mod receipt;
#[cfg(feature = "schema")]
pub mod schema;
