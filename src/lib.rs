//! Poet — AI-First Word Document (.docx) Automation CLI.
//!
//! A Rust rebuild of the Python `words` CLI. The Python original is the
//! behavioral spec; deviations are recorded as ADRs under `docs/adr/`.
//!
//! Layering (see `AGENTS.md`): `app` is wiring only, `commands` holds command
//! functions with the one true signature, `core` holds helpers/engine code,
//! `models` holds the output payload enum.

// Docs are mandatory in strict builds. Under the `dev` feature the gates
// relax (adr/0015) so an in-flight command can be compiled and run to capture
// a real envelope before its docs exist; `build.rs` rejects `dev` + release,
// so an undocumented binary never ships.
#![cfg_attr(not(feature = "dev"), deny(missing_docs))]

pub mod app;
pub mod commands;
pub mod core;
pub mod howto;
pub mod models;

/// Re-export of the .docx engine so integration tests (and downstream
/// tooling) can inspect document internals without a matching version pin.
pub use docx_rs;
