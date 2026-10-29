//! Authenticated stdio bridge for the already-running HTTP MCP service.

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn bridge_uses_same_service_no_redirect_or_reinit() {
        assert!(false, "RED: stdio bridge transport is not implemented yet");
    }

    #[tokio::test]
    async fn bridge_parallel_ids_cancel_and_eof() {
        assert!(false, "RED: stdio bridge forwarding is not implemented yet");
    }
}
