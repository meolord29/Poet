//! Compile-time gates (adr/0015): every command fn in `src/commands/` is
//! documented by example, tested, integration-covered, and scenario-featured.
//! A miss aborts the build — no binary is produced.
//!
//! A "command" is any `pub fn` in `src/commands/` whose return type ends in
//! `Result`. Helpers must live in `core/`, never in `commands/`. The command
//! set is derived from signatures, so adding a command auto-enrolls it in
//! every gate below — no registry to forget (carpenter `build.rs` port).
//!
//! For each command fn `<module>::<name>` a strict build requires:
//! - a worked-example file at `docs/examples/<module>/<name>.md`
//!   (captured real envelopes, authored via `--capture-example`); AND
//! - a paired `#[test] fn <name>_*` in the same module; AND
//! - an integration invocation in `tests/` (the executable path must cover
//!   every command — a Poet addition beyond carpenter); AND
//! - the scenario floor: at least one `examples/*.md`, each invoking ≥
//!   [`MIN_DISTINCT_FNS`] distinct command fns (adr/0013 analog).
//!
//! Additionally every `Data` variant must appear in `src/models/examples.rs`
//! (the envelope smoke test's registry), so a new payload shape cannot merge
//! without an example.
//!
//! Under the `dev` feature (adr/0015) these gates are skipped and
//! `#![deny(missing_docs)]` is relaxed, so a command can be compiled and run
//! to capture a real envelope before its atom/test exist. `dev` + `release`
//! is rejected — no relaxed binary ships.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/commands");
    println!("cargo:rerun-if-changed=src/models/data.rs");
    println!("cargo:rerun-if-changed=src/models/examples.rs");
    println!("cargo:rerun-if-changed=docs/examples");
    println!("cargo:rerun-if-changed=examples");
    println!("cargo:rerun-if-changed=tests");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // The `dev` feature (adr/0015) relaxes the gates below for the authoring
    // loop. It must never ship: a release binary built with `dev` would
    // bypass the self-documentation contract and the scenario floor.
    let dev = std::env::var_os("CARGO_FEATURE_DEV").is_some();
    if dev && std::env::var("PROFILE").as_deref() == Ok("release") {
        eprintln!(
            "error: the `dev` feature relaxes the doc/example/scenario gates \
             and must not be used in a release build (adr/0015)"
        );
        std::process::exit(1);
    }
    if dev {
        println!("cargo:warning=dev build: doc/example/scenario gates relaxed (adr/0015)");
    } else {
        strict_gates();
    }
}

/// All strict gates: accumulate every violation, then fail the build.
fn strict_gates() {
    let mut errors: Vec<String> = Vec::new();
    // Known command set as `<stem>::<name>` (signature-derived), reused by the
    // scenario gate to resolve `poet <category> <action>` invocations.
    let mut known: HashSet<String> = HashSet::new();
    let mut per_file: Vec<(String, Vec<String>)> = Vec::new();

    for file in walk_rs(Path::new("src/commands")) {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(parsed) = syn::parse_file(&src) else {
            errors.push(format!(
                "{}: not parseable by syn (gates cannot verify it)",
                file.display()
            ));
            continue;
        };
        let (cmds, tests) = scan(&parsed.items);
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        for name in &cmds {
            known.insert(format!("{stem}::{name}"));
        }
        for name in &cmds {
            // Gate A — example atom, keyed by the clap action name (kebab):
            // `set_level` fn → `set-level.md`, matching what users type.
            let action = name.replace('_', "-");
            let example = Path::new("docs/examples")
                .join(&stem)
                .join(format!("{action}.md"));
            if !example.exists() {
                errors.push(format!(
                    "commands/{stem}.rs:{name}: missing example file {} \
                     (capture one with `--capture-example` under a dev build, adr/0015)",
                    example.display()
                ));
            }
            // Gate B — paired in-module test.
            let prefix = format!("{name}_");
            if !tests.iter().any(|t| t.starts_with(&prefix)) {
                errors.push(format!(
                    "commands/{stem}.rs:{name}: missing #[test] fn {name}_* in the same module"
                ));
            }
        }
        per_file.push((stem, cmds));
    }

    // Gate C — integration coverage: the `<category>` + `<action>` string pair
    // must appear in some tests/*.rs invocation (actions are kebab-case in
    // argv, matching clap's rendering).
    let integration = all_tests_text();
    for (stem, cmds) in &per_file {
        for name in cmds {
            let category_token = format!("\"{stem}\"");
            let action = name.replace('_', "-");
            let action_token = format!("\"{action}\"");
            let covered = integration
                .iter()
                .any(|text| text.contains(&category_token) && text.contains(&action_token));
            if !covered {
                errors.push(format!(
                    "commands/{stem}.rs:{name}: no integration coverage in tests/ \
                     (invoke `\"{stem}\"` and `\"{name}\"` in a tests/*.rs case)"
                ));
            }
        }
    }

    gate_scenarios(&known, &mut errors);
    gate_data_examples(&mut errors);

    if errors.is_empty() {
        return;
    }
    for e in &errors {
        eprintln!("{e}");
    }
    panic!(
        "command gates failed with {} violation(s) (adr/0015)",
        errors.len()
    );
}

/// Minimum number of distinct command fns a scenario file must invoke.
const MIN_DISTINCT_FNS: usize = 3;

/// Global value-taking flags whose following argument must be skipped when
/// parsing `poet` invocations (dev-only today; release scenarios lead with
/// the category token).
const GLOBAL_VALUE_FLAGS: &[&str] = &["--capture-example", "--dev-home"];

/// The scenario floor: at least one `examples/*.md` must exist, and each must
/// invoke ≥ [`MIN_DISTINCT_FNS`] distinct known command fns. Only fenced
/// ```sh / ```bash blocks are parsed, so `poet` lines inside JSON/envelope
/// fences are ignored. Unknown invocations are reported by name — a typo in a
/// scenario must not silently stop counting coverage.
fn gate_scenarios(known: &HashSet<String>, errors: &mut Vec<String>) {
    let dir = Path::new("examples");
    let mut files = walk_md_flat(dir);
    files.sort();
    if files.is_empty() {
        errors.push("examples/: no scenario files (>=1 required, adr/0015)".to_string());
        return;
    }
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        let name = f
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut seen: HashSet<String> = HashSet::new();
        let mut unknown: Vec<String> = Vec::new();
        let mut in_fence = false;
        let mut fence_lang = String::new();
        for line in text.lines() {
            let t = line.trim_start();
            if t.starts_with("```") {
                if !in_fence {
                    in_fence = true;
                    fence_lang = t
                        .strip_prefix("```")
                        .unwrap_or("")
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string();
                } else {
                    in_fence = false;
                    fence_lang.clear();
                }
                continue;
            }
            if !in_fence || !(fence_lang == "sh" || fence_lang == "bash") {
                continue;
            }
            let Some(key) = parse_invocation(line, known) else {
                continue;
            };
            if known.contains(&key) {
                seen.insert(key);
            } else if !unknown.contains(&key) {
                unknown.push(key);
            }
        }
        if seen.len() < MIN_DISTINCT_FNS {
            errors.push(format!(
                "examples/{name}: references only {} distinct command fn(s) \
                 (>={MIN_DISTINCT_FNS} required, adr/0015)",
                seen.len()
            ));
        }
        for u in &unknown {
            errors.push(format!(
                "examples/{name}: unknown invocation `{u}` (not a known command fn)"
            ));
        }
    }
}

/// Parse one `poet …` line into a `<category>::<fn>` key. Clap renders
/// multi-word actions kebab-case (`set-range`) while fn names are snake
/// (`set_range`), so the second token is normalized. Returns `None` for
/// non-invocations.
fn parse_invocation(line: &str, known: &HashSet<String>) -> Option<String> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    if toks.first() != Some(&"poet") {
        return None;
    }
    let rest = strip_globals(&toks[1..]);
    let category = *rest.first()?;
    let action = rest.get(1)?;
    let snake = action.replace('-', "_");
    let key = format!("{category}::{snake}");
    if known.contains(&key) {
        return Some(key);
    }
    None
}

/// Drop leading global flags (and the value of any value-taking one) so the
/// first returned token is the command category.
fn strip_globals<'a>(toks: &'a [&'a str]) -> Vec<&'a str> {
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if !t.starts_with('-') {
            break;
        }
        if t.contains('=') {
            i += 1;
        } else if GLOBAL_VALUE_FLAGS.contains(&t) {
            i += 2;
        } else {
            i += 1;
        }
    }
    toks[i..].to_vec()
}

/// Gate E: every `Data` variant must appear in `src/models/examples.rs` —
/// the envelope smoke test's registry — so a new payload shape cannot merge
/// without a pinned example.
fn gate_data_examples(errors: &mut Vec<String>) {
    let Ok(data_src) = std::fs::read_to_string("src/models/data.rs") else {
        errors.push("src/models/data.rs: unreadable".to_string());
        return;
    };
    let Ok(parsed) = syn::parse_file(&data_src) else {
        errors.push("src/models/data.rs: not parseable by syn".to_string());
        return;
    };
    let Ok(examples_src) = std::fs::read_to_string("src/models/examples.rs") else {
        errors.push("src/models/examples.rs: unreadable".to_string());
        return;
    };
    for item in &parsed.items {
        let syn::Item::Enum(e) = item else {
            continue;
        };
        if e.ident != "Data" {
            continue;
        }
        for variant in &e.variants {
            let name = variant.ident.to_string();
            if !examples_src.contains(&name) {
                errors.push(format!(
                    "models/data.rs:{name}: no example in src/models/examples.rs \
                     (the envelope smoke test cannot cover it, adr/0015)"
                ));
            }
        }
    }
}

/// Contents of every `tests/*.rs` file (Gate C's corpus).
fn all_tests_text() -> Vec<String> {
    let mut out = Vec::new();
    for file in walk_rs(Path::new("tests")) {
        if let Ok(text) = std::fs::read_to_string(&file) {
            out.push(text);
        }
    }
    out
}

/// Recursively collect command fns (`pub fn` returning `Result`) and
/// `#[test]` fn names, including nested modules (inline test mods).
fn scan(items: &[syn::Item]) -> (Vec<String>, Vec<String>) {
    let mut cmds = Vec::new();
    let mut tests = Vec::new();
    for item in items {
        match item {
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                if f.attrs.iter().any(|a| a.path().is_ident("test")) {
                    tests.push(name.clone());
                }
                if is_public(f) && returns_result(&f.sig) {
                    // Raw identifiers (`r#move`) surface as clap actions
                    // without the `r#` prefix (`move`).
                    let stripped = name.strip_prefix("r#").unwrap_or(&name).to_string();
                    cmds.push(stripped);
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let (c, t) = scan(nested);
                    cmds.extend(c);
                    tests.extend(t);
                }
            }
            _ => {}
        }
    }
    (cmds, tests)
}

fn is_public(f: &syn::ItemFn) -> bool {
    matches!(f.vis, syn::Visibility::Public(_))
}

fn returns_result(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    matches!(peel(ty), syn::Type::Path(tp) if tp.path.segments.last().map(|s| s.ident == "Result").unwrap_or(false))
}

fn peel(mut ty: &syn::Type) -> &syn::Type {
    while let syn::Type::Paren(p) = ty {
        ty = &p.elem;
    }
    ty
}

fn walk_rs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_rs(&p));
        } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

/// Top-level `examples/*.md` files (scenarios are flat — one per workflow).
fn walk_md_flat(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_file() && p.extension().map(|x| x == "md").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}
