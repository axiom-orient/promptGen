//! Pure image-artifact validation and hashing primitives shared by adapters.
//!
//! This module owns the strict PNG and SHA-256 proof used before any image is
//! published. It has no provider, process, or filesystem side effects beyond
//! reading a path explicitly supplied by a caller.

pub mod png;
pub mod sha256;
