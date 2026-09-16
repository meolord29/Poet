//! Error model: one `thiserror` enum, one envelope code per variant.
//!
//! Words' `exceptions.py` defines exception classes without string codes; the
//! envelope `code` values there are mostly `""` (with one `"UNSUPPORTED"`).
//! Poet ports the classes to machine-readable codes (constitutions §3) so
//! agents can branch on failures; the envelope shape is unchanged.

/// The single error type for every command and core helper.
#[derive(Debug, thiserror::Error)]
pub enum PoetError {
    /// Session management failed (read/write/delete of `session.json`).
    #[error("session error: {0}")]
    Session(String),

    /// Input validation failed (bad level, unsupported format name, ...).
    #[error("validation error: {0}")]
    Validation(String),

    /// Metadata operations failed (phase 4).
    #[error("metadata error: {0}")]
    Metadata(String),

    /// Calculation/analysis operations failed (phase 4).
    #[error("calculation error: {0}")]
    Calculation(String),

    /// File operations failed (save/pack/IO).
    #[error("file error: {0}")]
    File(String),

    /// A referenced element (bookmark id, path, table) does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// A unique name is already taken or a state conflict occurred.
    #[error("conflict: {0}")]
    Conflict(String),

    /// The operation requires an open document (or none to be open).
    #[error("document state error: {0}")]
    DocumentState(String),

    /// The requested capability is not supported by the engine.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// Command exists but its body has not been ported yet (phased rebuild).
    #[error("not implemented: {0}")]
    NotImplemented(String),

    /// An unexpected internal condition; the message carries context.
    #[error("internal error: {0}")]
    Internal(String),
}

impl PoetError {
    /// The stable machine-readable code used in the error envelope.
    pub fn code(&self) -> &'static str {
        match self {
            PoetError::Session(_) => "session_error",
            PoetError::Validation(_) => "validation_error",
            PoetError::Metadata(_) => "metadata_error",
            PoetError::Calculation(_) => "calculation_error",
            PoetError::File(_) => "file_error",
            PoetError::NotFound(_) => "not_found",
            PoetError::Conflict(_) => "conflict",
            PoetError::DocumentState(_) => "document_state",
            PoetError::Unsupported(_) => "unsupported",
            PoetError::NotImplemented(_) => "not_implemented",
            PoetError::Internal(_) => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PoetError;

    #[test]
    fn code_covers_every_variant_and_is_stable() {
        let cases: Vec<(PoetError, &str)> = vec![
            (PoetError::Session("s".into()), "session_error"),
            (PoetError::Validation("v".into()), "validation_error"),
            (PoetError::Metadata("m".into()), "metadata_error"),
            (PoetError::Calculation("c".into()), "calculation_error"),
            (PoetError::File("f".into()), "file_error"),
            (PoetError::NotFound("n".into()), "not_found"),
            (PoetError::Conflict("x".into()), "conflict"),
            (PoetError::DocumentState("d".into()), "document_state"),
            (PoetError::Unsupported("u".into()), "unsupported"),
            (PoetError::NotImplemented("ni".into()), "not_implemented"),
            (PoetError::Internal("i".into()), "internal"),
        ];
        for (err, code) in cases {
            assert_eq!(err.code(), code);
        }
    }
}
