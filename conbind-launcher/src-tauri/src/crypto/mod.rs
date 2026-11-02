//! Application crypto primitives.
//!
//! - [`cipher`] / [`keystore`]: Layer 3 encryption-at-rest.
//!
//! Spec: `docs/superpowers/specs/2026-05-03-encryption-at-rest.md`.

pub mod cipher;
pub mod hmac;
pub mod keystore;

/// A fresh random master key for tests. Tests never use a fixed key: a literal
/// key reads as a hard-coded secret to code scanners, and nothing depends on its value.
#[cfg(test)]
pub fn test_master_key() -> [u8; 32] {
    rand::random()
}
