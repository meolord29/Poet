//! Session persistence: which document is "open" between processes.
//!
//! Mirrors Words' `core/session.py`: the session file records the active
//! document so chained CLI invocations (or an agent's next call) auto-open
//! it. The repository path is explicit — production resolves `POET_HOME` or
//! `~/.poet`, tests pass a temp-dir path — so tests never touch `~/.poet`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::error::PoetError;

/// The persisted session record (shape identical to Words).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Absolute or relative path of the active document.
    pub path: PathBuf,
    /// Document format, always `"docx"` today.
    pub format: String,
    /// Reserved for future section tracking; `None` today.
    #[serde(default)]
    pub active_section: Option<String>,
}

/// Reads/writes `session.json` at an explicit location.
#[derive(Debug, Clone)]
pub struct SessionRepository {
    /// Full path of the `session.json` file.
    pub session_file: PathBuf,
}

impl SessionRepository {
    /// Repository rooted at an explicit directory (the dir containing
    /// `session.json`).
    pub fn at_dir(dir: &Path) -> Self {
        SessionRepository {
            session_file: dir.join("session.json"),
        }
    }

    /// Production repository: `$POET_HOME/session.json` or
    /// `$HOME/.poet/session.json`.
    pub fn from_env() -> Self {
        let home = std::env::var_os("POET_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".poet")))
            .unwrap_or_else(|| PathBuf::from(".poet"));
        SessionRepository::at_dir(&home)
    }

    /// Load the session, returning `None` when absent or unreadable
    /// (Words treats a corrupt session as no session).
    pub fn get(&self) -> Option<Session> {
        let raw = fs::read_to_string(&self.session_file).ok()?;
        serde_json::from_str(&raw).ok()
    }

    /// Persist the session (creates the directory), compact JSON like Words.
    pub fn save(&self, session: &Session) -> Result<(), PoetError> {
        let dir = self
            .session_file
            .parent()
            .ok_or_else(|| PoetError::Session("session path has no parent".into()))?;
        fs::create_dir_all(dir)
            .map_err(|e| PoetError::Session(format!("Failed to save session: {e}")))?;
        let json = serde_json::to_string(session)
            .map_err(|e| PoetError::Session(format!("Failed to save session: {e}")))?;
        fs::write(&self.session_file, json)
            .map_err(|e| PoetError::Session(format!("Failed to save session: {e}")))
    }

    /// Remove the session file; deleting a missing session is a no-op.
    pub fn delete(&self) -> Result<(), PoetError> {
        if self.session_file.exists() {
            fs::remove_file(&self.session_file)
                .map_err(|e| PoetError::Session(format!("Failed to delete session: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionRepository};
    use std::path::PathBuf;

    fn repo(tmp: &tempfile::TempDir) -> SessionRepository {
        SessionRepository::at_dir(tmp.path())
    }

    #[test]
    fn save_get_delete_round_trips() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(&tmp);
        assert!(r.get().is_none());
        let session = Session {
            path: PathBuf::from("/tmp/x.docx"),
            format: "docx".into(),
            active_section: None,
        };
        r.save(&session).expect("save");
        assert_eq!(r.get(), Some(session));
        r.delete().expect("delete");
        assert!(r.get().is_none());
    }

    #[test]
    fn delete_missing_session_is_ok() {
        let tmp = tempfile::tempdir().expect("tempdir");
        repo(&tmp).delete().expect("delete missing is noop");
    }

    #[test]
    fn corrupt_session_reads_as_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(&tmp);
        std::fs::write(&r.session_file, "not json").expect("write");
        assert!(r.get().is_none());
    }

    #[test]
    fn save_creates_missing_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = SessionRepository::at_dir(&tmp.path().join("nested/poet"));
        let session = Session {
            path: PathBuf::from("a.docx"),
            format: "docx".into(),
            active_section: None,
        };
        r.save(&session).expect("save nested");
        assert!(r.get().is_some());
    }
}
