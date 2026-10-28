//! Tier 2 extension context lifecycle — worker + view state machines.
//! Each extension has two independent ContextMachine instances (worker and view),
//! both parameterised by ContextRole and sharing the same state alphabet.
//! See docs/superpowers/plans/2026-04-21-tier2-worker-view-split.md

pub mod context;
pub mod emitter;
pub mod manager;
pub mod ticker;
pub mod types;
pub mod wire;

pub use manager::ExtensionRuntimeManager;
pub use types::*;

// Tauri event names shared between commands and ticker
pub const EVENT_MOUNT: &str = "conbind:iframe:mount";
pub const EVENT_UNMOUNT: &str = "conbind:iframe:unmount";
pub const EVENT_DELIVER: &str = "conbind:iframe:deliver";
pub const EVENT_DEGRADED: &str = "conbind:iframe:degraded";
pub const EVENT_RECOVERED: &str = "conbind:iframe:recovered";

#[cfg(test)]
mod proptests;
