use ontoforge_rmcp_shell_spike::bridge;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let Some(url) = std::env::args().nth(1) else {
        return std::process::ExitCode::FAILURE;
    };
    let Ok(token) = std::env::var("SPIKE_TOKEN") else {
        return std::process::ExitCode::FAILURE;
    };
    let stop = CancellationToken::new();
    let signal = stop.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        signal.cancel();
    });
    match bridge(tokio::io::stdin(), tokio::io::stdout(), &url, token, stop).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => {
            eprintln!("bridge failed");
            std::process::ExitCode::FAILURE
        }
    }
}
