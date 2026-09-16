//! Binary entry point; all behavior lives in the library.

fn main() -> std::process::ExitCode {
    poet::app::run(std::env::args().skip(1))
}
