//! Dev-build-only commands (adr/0015): the QA sandbox lifecycle. Compiled
//! exclusively under the `dev` feature, so none of this surface reaches a
//! release binary, `poet howto`, or the QA agent's release-parity probes.
//! All mechanics live in [`crate::core::dev`]; this file is the signature
//! wrapper, per the layering rules in `AGENTS.md`.

use crate::core::Ctx;
use crate::core::error::PoetError;
use crate::models::data::Data;

/// `poet dev` sub-actions.
#[derive(Debug, clap::Subcommand)]
pub enum DevAction {
    /// Create `.sandbox/` (plus its session home) under the current
    /// directory — idempotent.
    Setup(SetupArgs),
    /// Remove `.sandbox/` under the current directory — idempotent; runs
    /// even after failed iterations.
    Clean(CleanArgs),
}

/// Arguments for `dev setup` (none — the sandbox location is fixed).
#[derive(Debug, clap::Args)]
pub struct SetupArgs {}

/// Arguments for `dev clean` (none — the sandbox location is fixed).
#[derive(Debug, clap::Args)]
pub struct CleanArgs {}

/// `poet dev setup`: create the validation sandbox under the current
/// directory. The QA agent's only way to obtain a workspace (it holds no
/// filesystem permissions of its own, adr/0015).
pub fn setup(_ctx: &Ctx, _args: &SetupArgs) -> Result<Data, PoetError> {
    crate::core::dev::setup_at(&crate::core::dev::cwd()?)
}

/// `poet dev clean`: remove the validation sandbox under the current
/// directory, session state included.
pub fn clean(_ctx: &Ctx, _args: &CleanArgs) -> Result<Data, PoetError> {
    crate::core::dev::clean_at(&crate::core::dev::cwd()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dev::{clean_at, setup_at};

    #[test]
    fn setup_creates_sandbox_idempotently() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let Data::DevSetup { created, .. } = setup_at(tmp.path()).expect("setup") else {
            panic!("expected DevSetup");
        };
        assert!(created);
        let Data::DevSetup { created, .. } = setup_at(tmp.path()).expect("setup 2") else {
            panic!("expected DevSetup");
        };
        assert!(!created);
    }

    #[test]
    fn clean_removes_sandbox_idempotently() {
        let tmp = tempfile::tempdir().expect("tempdir");
        setup_at(tmp.path()).expect("setup");
        let Data::DevClean { removed, .. } = clean_at(tmp.path()).expect("clean") else {
            panic!("expected DevClean");
        };
        assert!(removed);
        let Data::DevClean { removed, .. } = clean_at(tmp.path()).expect("clean 2") else {
            panic!("expected DevClean");
        };
        assert!(!removed);
    }
}
