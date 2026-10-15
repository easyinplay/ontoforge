use std::process::ExitCode;

use clap::Parser;
use ontoforge::cli::Cli;

/// The pipeline runs on a large stack: debug builds on Windows have a 1 MiB main
/// thread stack, which nested YAML can exhaust.
const STACK_SIZE: usize = 32 << 20;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let handle = std::thread::Builder::new()
        .name("ontoforge-main".into())
        .stack_size(STACK_SIZE)
        .spawn(move || ontoforge::run(cli));
    if let Ok(Ok(code)) = handle.map(std::thread::JoinHandle::join) {
        ExitCode::from(code)
    } else {
        eprintln!("internal error: ontoforge worker thread failed");
        ExitCode::from(101)
    }
}
