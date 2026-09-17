//! The AI-oriented reference (`poet howto`, and no-subcommand output).
//!
//! Ported from Words' `HOWTO_PROMPT` with `words`→`poet` /
//! `~/.words`→`~/.poet` substitutions, the install section rewritten for
//! cargo, calc flags corrected to Poet's actual `--id`/`--index`, and the
//! summary tables aligned to Poet's real command set (adr-adjacent doc
//! fixes recorded in PLAN.md; structure/coverage identical to Words').

/// The howto text printed by `poet howto` and when no subcommand is given.
pub const HOWTO: &str = include_str!("howto.md");

#[cfg(test)]
mod tests {
    use super::HOWTO;

    #[test]
    fn howto_is_rebranded_and_complete() {
        assert!(HOWTO.starts_with("# Poet - AI-First Word Document Automation CLI"));
        assert!(!HOWTO.contains("Words"), "stale CLI name");
        assert!(!HOWTO.contains("words"), "stale CLI name");
        assert!(!HOWTO.contains("~/.words"));
        assert!(!HOWTO.contains("python-docx"));
        assert!(!HOWTO.contains("--table-id"), "flag drift from Words' docs");
        assert!(HOWTO.contains("poet batch run script.json"));
        assert!(HOWTO.contains("calc"));
        assert!(HOWTO.contains("meta"));
        assert!(HOWTO.contains("batch"));
        assert!(!HOWTO.contains("\"filter\""), "phantom transform op");
    }
}
