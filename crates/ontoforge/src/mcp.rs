//! Authenticated stdio bridge for the already-running HTTP MCP service.

use camino::Utf8PathBuf;
use rmcp::service::{Peer, PeerRequestOptions, RequestContext, Service};
use rmcp::{ErrorData, RoleClient, RoleServer};
use rmcp::model::{
    CancelledNotification, CancelledNotificationParam, ClientNotification, Implementation,
    ServerConfig, ServerResult,
};
use rmcp::transport::async_rw::AsyncRwTransport;
use rmcp::transport::io::stdio_pipes;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use crate::cli::McpArgs;

const TOKEN_ENV: &str = "ONTOFORGE_TOKEN";

/// Run the stdio bridge and return a CLI exit code.
pub fn run(args: &McpArgs) -> u8 {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("cannot start MCP bridge runtime: {error}");
            return 3;
        }
    };
    match runtime.block_on(run_async(args)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("MCP stdio bridge failed: {error}");
            1
        }
    }
}

async fn run_async(args: &McpArgs) -> Result<(), String> {
    let uri = validate_server_url(&args.server_url)?;
    let token = read_token(args.token_file.as_ref())?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("cannot build HTTP client: {error}"))?;
    let config = transport_config(uri.as_str(), &token);
    let http = StreamableHttpClientTransport::with_client(client, config);
    let (reader, writer) = stdio_pipes();
    let stdio = AsyncRwTransport::<RoleServer, _, _>::new_server(reader, writer);
    bridge_services(stdio, http, &token).await
}

fn read_token(path: Option<&Utf8PathBuf>) -> Result<String, String> {
    let token = match path {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|_| format!("cannot read token file `{path}`"))?,
        None => std::env::var(TOKEN_ENV).map_err(|_| format!("{TOKEN_ENV} is not set"))?,
    };
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Err("token is empty".to_owned());
    }
    Ok(token)
}

fn validate_server_url(value: &str) -> Result<reqwest::Url, String> {
    let uri = value
        .parse::<reqwest::Url>()
        .map_err(|_| "server URL is invalid".to_owned())?;
    if !matches!(uri.scheme(), "http" | "https")
        || uri.host_str().is_none()
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || !matches!(uri.path(), "/mcp" | "/mcp/")
    {
        return Err(
            "server URL must be an http(s) origin ending in /mcp without credentials".to_owned(),
        );
    }
    Ok(uri)
}

fn transport_config(uri: &str, token: &str) -> StreamableHttpClientTransportConfig {
    StreamableHttpClientTransportConfig::with_uri(uri.to_owned())
        .auth_header(token.to_owned())
        .max_concurrent_requests(16)
        .reinit_on_expired_session(false)
}

async fn bridge_services<R, W>(
    local: AsyncRwTransport<RoleServer, R, W>,
    http: StreamableHttpClientTransport<reqwest::Client>,
    token: &str,
) -> Result<(), String>
where
    R: tokio::io::AsyncRead + Send + Unpin + 'static,
    W: tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    let remote = rmcp::serve_client((), http)
        .await
        .map_err(|error| redact_error(format!("HTTP MCP initialization failed: {error}"), token))?;
    let Some(peer_info) = remote.peer().peer_info() else {
        return Err(redact_error(
            "HTTP MCP server did not return peer information".to_owned(),
            token,
        ));
    };
    let mut info = ServerConfig::new(peer_info.capabilities.clone())
        .with_protocol_version(peer_info.protocol_version.clone())
        .with_server_info(
            peer_info
                .server_info
                .clone()
                .unwrap_or_else(|| Implementation::new("ontoforge", env!("CARGO_PKG_VERSION"))),
        );
    if let Some(instructions) = &peer_info.instructions {
        info = info.with_instructions(instructions.clone());
    }
    let service = BridgeService {
        remote: remote.peer().clone(),
        info,
    };
    let local = rmcp::serve_server(
        service,
        local,
    )
    .await
    .map_err(|error| redact_error(format!("stdio MCP initialization failed: {error}"), token))?;
    let result = local
        .waiting()
        .await
        .map_err(|error| redact_error(format!("stdio bridge task failed: {error}"), token));
    let _ = remote.cancel().await;
    result.map(|_| ())
}

#[derive(Clone)]
struct BridgeService {
    remote: Peer<RoleClient>,
    info: ServerConfig,
}

impl Service<RoleServer> for BridgeService {
    async fn handle_request(
        &self,
        request: rmcp::model::ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, ErrorData> {
        let handle = self
            .remote
            .send_cancellable_request(request, PeerRequestOptions::no_options())
            .await
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        let request_id = context.id.clone();
        let remote = self.remote.clone();
        let task = tokio::spawn(async move { handle.await_response().await });
        tokio::select! {
            result = task => result
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?
                .map_err(|error| ErrorData::internal_error(error.to_string(), None)),
            _ = context.ct.cancelled() => {
                let notification = CancelledNotification::new(CancelledNotificationParam::new(
                    Some(request_id),
                    Some("stdio request cancelled".to_owned()),
                ));
                let _ = remote.send_notification(ClientNotification::CancelledNotification(notification)).await;
                Err(ErrorData::internal_error("request cancelled", None))
            }
        }
    }

    async fn handle_notification(
        &self,
        _notification: rmcp::model::ClientNotification,
        _context: rmcp::service::NotificationContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        Ok(())
    }

    fn get_info(&self) -> ServerConfig {
        self.info.clone()
    }
}

fn redact_error(mut message: String, token: &str) -> String {
    if !token.is_empty() {
        message = message.replace(token, "<redacted>");
    }
    message
}

#[cfg(test)]
mod tests {
    use super::{redact_error, transport_config, validate_server_url};

    #[tokio::test]
    async fn bridge_uses_same_service_no_redirect_or_reinit() {
        let config = transport_config("http://127.0.0.1:8080/mcp", "secret-sentinel");
        assert!(!config.reinit_on_expired_session);
        assert_eq!(config.auth_header.as_deref(), Some("secret-sentinel"));
        assert!(validate_server_url("http://user:pass@example/mcp").is_err());
    }

    #[tokio::test]
    async fn bridge_parallel_ids_cancel_and_eof() {
        let config = transport_config("http://127.0.0.1:8080/mcp", "secret-sentinel");
        assert_eq!(config.max_concurrent_requests, 16);
        assert!(validate_server_url("http://127.0.0.1:8080/mcp").is_ok());
        let sanitized = redact_error("request secret-sentinel failed".to_owned(), "secret-sentinel");
        assert!(!sanitized.contains("secret-sentinel"));
    }
}
