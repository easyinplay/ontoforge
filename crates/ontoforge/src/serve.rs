//! The HTTP host command and its injectable shutdown boundary.

use std::future::Future;

use crate::cli::ServeArgs;

pub fn run(_args: &ServeArgs) -> u8 {
    3
}

pub async fn run_with_shutdown(
    _args: &ServeArgs,
    _shutdown: impl Future<Output = ()> + Send + 'static,
) -> u8 {
    3
}

#[cfg(test)]
#[path = "serve.test.rs"]
mod tests;
