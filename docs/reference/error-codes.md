# Reference: Error & Exit Codes

Every Poet invocation prints exactly one JSON envelope to stdout:

```json
{"status": "ok", "message": "...", "data": {...}}                       // exit 0
{"status": "error", "message": "...", "code": "...", "details": {}}     // exit 1
```

`code` is a stable machine-readable string — agents branch on it, never on the
message text. Clap usage errors (unknown command, bad flags) print plain text
and exit 2. The raw-text commands `poet howto` and `poet completions` exit 0.

The canonical source is `PoetError::code()` in `src/core/error.rs`; a unit
test pins that every code below exists in the enum (and vice versa).

## Codes

| Code | Variant | Meaning / typical sources |
|------|---------|---------------------------|
| `session_error` | `Session` | Reading, writing, or deleting `session.json` failed. |
| `validation_error` | `Validation` | Bad input: unknown unit/orientation/style-type, invalid JSON argument, malformed `--range`, unknown batch template/action, missing required batch key. |
| `metadata_error` | `Metadata` | A metadata (meta) operation failed. |
| `calculation_error` | `Calculation` | Calc failures: unknown operator/aggregate function/operation type, mixed-type columns, expression errors (missing column, type mismatch), missing transform column. |
| `file_error` | `File` | File I/O: unreadable/unwritable paths, zip repack failures, malformed .docx packages. |
| `not_found` | `NotFound` | Missing file, missing bookmark id (`No table with id 'x'`), table index out of range, document has no tables. |
| `conflict` | `Conflict` | A unique name is taken or a state conflict occurred. |
| `document_state` | `DocumentState` | The operation needs an open document (`No document is open`). |
| `unsupported` | `Unsupported` | The engine cannot do it (e.g. `document export ... pdf`). |
| `not_implemented` | `NotImplemented` | Reserved for phased rebuilds; release builds never return it. |
| `internal` | `Internal` | Unexpected internal condition — messages carry context; please report. |
