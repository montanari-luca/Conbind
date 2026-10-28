//! Application crypto primitives.
//!
//! - [`cipher`] / [`keystore`]: Layer 3 encryption-at-rest.
//!
//! Spec: `docs/superpowers/specs/2026-05-03-encryption-at-rest.md`.

pub mod cipher;
pub mod hmac;
pub mod keystore;
