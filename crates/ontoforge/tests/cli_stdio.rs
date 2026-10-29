use std::process::{Command, Stdio};

#[test]
fn stdio_real_process_describe_query_action_dryrun_history() {
    assert!(false, "RED: real stdio process contract is not implemented yet");
}

#[test]
fn stdio_server_absent_bad_token_redirect_and_expired_session() {
    assert!(false, "RED: stdio rejection contract is not implemented yet");
}

#[test]
fn stdio_long_query_cancel_eof_and_shutdown() {
    assert!(false, "RED: stdio shutdown contract is not implemented yet");
}

#[allow(dead_code)]
fn child() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_ontoforge"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ontoforge")
}
