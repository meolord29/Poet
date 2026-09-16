//! Core helpers: error model, output contract, session, document engine,
//! bookmark addressing. Business logic lives here — never in `commands`.

pub mod bookmark;
pub mod document;
pub mod error;
pub mod output;
pub mod session;

use std::cell::RefCell;

use crate::core::document::DocumentManager;
use crate::core::session::SessionRepository;

/// Per-invocation context built once in `app::run` and passed by reference.
///
/// The open document is behind a `RefCell` so command functions can mutate it
/// through `&Ctx` (the one command signature, `AGENTS.md` §1); the CLI is
/// single-threaded per invocation, so borrows never contend. `Clone` exists
/// for tests that drive several invocations from one root.
#[derive(Clone)]
pub struct Ctx {
    /// Session repository (auto-open source for chained invocations).
    pub session: SessionRepository,
    /// The open document, if any.
    pub doc: RefCell<Option<DocumentManager>>,
}

impl Ctx {
    /// Context with an explicit repository (tests) and nothing open.
    pub fn new(session: SessionRepository) -> Self {
        Ctx {
            session,
            doc: RefCell::new(None),
        }
    }

    /// Context rooted at an explicit directory, with session auto-open —
    /// the deterministic (env-free) variant of [`Ctx::from_env`] for tests.
    pub fn at_dir(dir: &std::path::Path) -> Self {
        let ctx = Ctx::new(SessionRepository::at_dir(dir));
        ctx.auto_open();
        ctx
    }

    /// Auto-open the session document if present. Failures are silently
    /// ignored, exactly like Words.
    fn auto_open(&self) {
        if let Some(session) = self.session.get()
            && session.path.exists()
        {
            let mut mgr = DocumentManager::new();
            if mgr.open(&session.path).is_ok() {
                *self.doc.borrow_mut() = Some(mgr);
            }
        }
    }

    /// Production context: repository from the environment, then auto-open
    /// the session document if present (Words' `get_context` behavior that
    /// makes `&&` chaining work across processes).
    pub fn from_env() -> Self {
        let ctx = Ctx::new(SessionRepository::from_env());
        ctx.auto_open();
        ctx
    }
}
