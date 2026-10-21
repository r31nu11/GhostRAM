//! Engine-independent domain types, quantities, errors, and report contracts for
//! GhostRAM.
//!
//! This crate must remain free of OS-specific collection, CLI, engine, and GUI
//! dependencies. Collection lives in `ghostram-probe`; presentation lives in
//! `ghostram-cli`.

pub mod diagnostic;
pub mod error;
pub mod quantity;
pub mod time;
