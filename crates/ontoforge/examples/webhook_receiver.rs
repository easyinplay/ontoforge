//! Independent HTTP receiver for the Phase 8 webhook end-to-end test.
//!
//! This is the RED stub: it binds a loopback port, publishes its target URL through a
//! ready file and answers every request with 204, but it writes no ledger record. The
//! end-to-end test therefore fails on a behavioural assertion (the receiver recorded no
//! requests) instead of on a build error.

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::routing::post;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "webhook-receiver",
    about = "Independent webhook receiver for the end-to-end test"
)]
struct Args {
    /// TCP port on 127.0.0.1; 0 lets the operating system choose one.
    #[arg(long, default_value_t = 0)]
    port: u16,
    /// JSONL ledger that receives one line per request.
    #[arg(long, value_name = "FILE")]
    ledger: PathBuf,
    /// File that receives the full target URL once the listener is bound.
    #[arg(long, value_name = "FILE")]
    ready_file: PathBuf,
    /// Answer the first attempt of every event with 503 and later attempts with 204.
    #[arg(long)]
    fail_first_per_event: bool,
}

async fn receive(
    ConnectInfo(_peer): ConnectInfo<SocketAddr>,
    _headers: HeaderMap,
    _uri: Uri,
    _body: Bytes,
) -> StatusCode {
    StatusCode::NO_CONTENT
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", args.port))
        .await
        .expect("bind the receiver on 127.0.0.1");
    let address = listener.local_addr().expect("receiver address");
    let target = format!("http://{address}/events");
    // The ready file is written through a temporary file so a reader never sees half a line.
    let temporary = args.ready_file.with_extension("tmp");
    std::fs::write(&temporary, target.as_bytes()).expect("write the ready file");
    std::fs::rename(&temporary, &args.ready_file).expect("publish the ready file");
    // The stub records nothing; it only creates the ledger so the test can tell an empty
    // ledger from a missing one.
    std::fs::File::create(&args.ledger).expect("create the ledger file");
    let app = Router::new().route("/events", post(receive));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve the receiver");
}
