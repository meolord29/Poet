//! Workspace automation tasks, invoked via `cargo xtask <task>`. Poet's build
//! gates live in the root `build.rs` (adr/0015), so xtask owns only the
//! version ladder (adr/0016) and the hooks drift test.

mod bump;
mod hooks;
mod paths;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "poet workspace tasks", bin_name = "cargo xtask")]
struct Xtask {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Advance the version ladder (adr/0016): mutate both manifests, rewrite
    /// version-bearing example atoms, refresh `Cargo.lock`. Git is left to the
    /// caller (release.yml commits and pushes the result).
    Bump {
        /// Which component to advance: `patch` (nightly merges), `minor`
        /// (promotion), `major` (reserved for critical/official changes).
        /// Mutually exclusive with `--to`.
        #[arg(value_enum)]
        part: Option<bump::Part>,
        /// Land an explicit X.Y.Z (the promotion bot: `main`'s minor+1
        /// exactly, even when the PR is reopened mid-ladder).
        #[arg(long, conflicts_with = "part")]
        to: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    match Xtask::parse().cmd {
        Cmd::Bump { part, to } => bump::run(part, to),
    }
}
