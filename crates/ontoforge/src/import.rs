//! `ontoforge import`: load a CSV or JSON file into an editOnly object type or link (STORE-06).

use crate::cli::ImportArgs;

/// Runs `ontoforge import` and returns the process exit code.
pub fn run(args: &ImportArgs) -> u8 {
    let _ = args;
    eprintln!("import is not implemented yet");
    1
}
