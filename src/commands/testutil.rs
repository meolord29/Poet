//! Shared test fixture: a `Ctx` rooted in a unique temp directory.
//!
//! Always compiled (no `cfg(test)`) so `tests/` integration binaries can use
//! it too; it has no dependencies beyond std. Production resolves the
//! session home from the environment; tests never touch `~/.poet` — they
//! build the `Ctx` directly with a temp-dir repository (see `AGENTS.md`
//! layout conventions). Directories are left behind deliberately: tests
//! neither assume nor require cleanup, matching carpenter's fixture style.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::Ctx;
use crate::core::session::SessionRepository;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Build a unique per-test context; returns the context and its temp root
/// (the directory holding `session.json`).
pub fn setup() -> (Ctx, PathBuf) {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("poet-test-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&dir).expect("create unique temp dir");
    let ctx = Ctx::new(SessionRepository::at_dir(&dir));
    (ctx, dir)
}
