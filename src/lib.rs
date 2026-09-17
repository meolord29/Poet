//! Poet — AI-First Word Document (.docx) Automation CLI.
//!
//! A Rust rebuild of the Python `words` CLI. The Python original is the
//! behavioral spec; deviations are recorded as ADRs under `docs/adr/`.
//!
//! Layering (see `AGENTS.md`): `app` is wiring only, `commands` holds command
//! functions with the one true signature, `core` holds helpers/engine code,
//! `models` holds the output payload enum.

#![deny(missing_docs)]

pub mod app;
pub mod commands;
pub mod core;
pub mod howto;
pub mod models;

/// Re-export of the .docx engine so integration tests (and downstream
/// tooling) can inspect document internals without a matching version pin.
pub use docx_rs;
