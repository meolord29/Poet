//! The one output contract: a single JSON envelope per invocation.
//!
//! Success: `{"status":"ok","message":"...","data":{...}}` (exit 0).
//! Failure: `{"status":"error","message":"...","code":"...","details":{}}`
//! (exit 1). Pretty-printed with 2-space indent, non-ASCII preserved —
//! matching Words' `json.dumps(..., indent=2, ensure_ascii=False)`.
//! This module is the only place envelopes are shaped (`AGENTS.md` §2).

use serde::Serialize;
use serde_json::Value;

use crate::core::error::PoetError;
#[doc(inline)]
pub use crate::models::data::Data;

/// A fully-shaped envelope ready for serialization.
#[derive(Debug, Serialize)]
pub struct Envelope {
    status: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

/// Build the success envelope for a command payload.
///
/// The top-level `message` mirrors the payload's own message (Words emits it
/// in both places); `document info` has no payload message, so it is empty.
pub fn ok(data: &Data) -> Envelope {
    Envelope {
        status: "ok",
        message: data.message().to_string(),
        data: Some(serde_json::to_value(data).unwrap_or(Value::Null)),
        code: None,
        details: None,
    }
}

/// Build the error envelope with the variant's stable code.
pub fn error(err: &PoetError) -> Envelope {
    Envelope {
        status: "error",
        message: err.to_string(),
        data: None,
        code: Some(err.code().to_string()),
        details: Some(Value::Object(serde_json::Map::new())),
    }
}

/// Serialize an envelope exactly as Words would print it.
pub fn to_json(envelope: &Envelope) -> String {
    // Rendering must never panic (`AGENTS.md` §2); emit a literal fallback
    // shaped like an error envelope if serialization somehow fails.
    let mut out = serde_json::to_string_pretty(envelope).unwrap_or_else(|_| {
        "{\n  \"status\": \"error\",\n  \"message\": \"serialization failed\",\n  \"code\": \"internal\",\n  \"details\": {}\n}"
            .to_string()
    });
    out.push('\n');
    out
}

/// Render a command result to (stdout text, is-error) in one step.
pub fn render(result: &Result<Data, PoetError>) -> (String, bool) {
    match result {
        Ok(data) => (to_json(&ok(data)), false),
        Err(err) => (to_json(&error(err)), true),
    }
}

#[cfg(test)]
mod tests {
    use super::{ok, render, to_json};
    use crate::core::error::PoetError;
    use crate::models::data::Data;

    #[test]
    fn ok_envelope_has_status_message_data() {
        let data = Data::DocumentClosed {
            message: "Document closed".into(),
        };
        let json = to_json(&ok(&data));
        assert!(json.contains("\"status\": \"ok\""));
        assert!(json.contains("\"message\": \"Document closed\""));
        assert!(json.contains("\"data\""));
        assert!(!json.contains("\"code\""));
        assert!(!json.contains("\"details\""));
    }

    #[test]
    fn error_envelope_has_code_and_details_and_exit_flag() {
        let (json, is_err) = render(&Err(PoetError::NotFound("nope".into())));
        assert!(is_err);
        assert!(json.contains("\"status\": \"error\""));
        assert!(json.contains("\"code\": \"not_found\""));
        assert!(json.contains("\"details\": {}"));
    }

    #[test]
    fn non_ascii_is_preserved_verbatim() {
        let data = Data::DocumentClosed {
            message: "Dokument geschlossen — 中文 ✓".into(),
        };
        let json = to_json(&ok(&data));
        assert!(json.contains("Dokument geschlossen — 中文 ✓"));
    }

    #[test]
    fn output_is_pretty_printed_with_two_space_indent() {
        let data = Data::DocumentClosed {
            message: "m".into(),
        };
        let json = to_json(&ok(&data));
        assert!(json.starts_with("{\n  \"status\": \"ok\",\n"));
    }
}
