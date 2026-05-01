use std::path::PathBuf;

#[test]
fn socket_path_joins_target_dir() {
    let root = PathBuf::from("/tmp/myproject");
    let sock = ctl::daemon::socket_path(&root);
    assert_eq!(sock, PathBuf::from("/tmp/myproject/target/ctl-daemon.sock"));
}

#[test]
fn socket_path_relative_path() {
    let root = PathBuf::from("myproject");
    let sock = ctl::daemon::socket_path(&root);
    assert!(sock.to_string_lossy().contains("target"));
    assert!(sock.to_string_lossy().ends_with("ctl-daemon.sock"));
}

#[tokio::test]
async fn check_liveness_nonexistent_socket() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("nonexistent_ctl_test.sock");
    let result = ctl::daemon::check_liveness(&missing).await;
    assert!(!result, "should return false for nonexistent socket");
}

#[tokio::test]
async fn check_liveness_with_real_socket() {
    let tmp = tempfile::tempdir().unwrap();
    let sock = tmp.path().join("alive.sock");
    let _server = ctl_daemon::ipc::IpcServer::bind(&sock).await.unwrap();
    let result = ctl::daemon::check_liveness(&sock).await;
    assert!(result, "should return true for listening socket");
}

#[tokio::test]
async fn check_ready_with_responding_server() {
    let tmp = tempfile::tempdir().unwrap();
    let sock = tmp.path().join("ready.sock");

    let server = ctl_daemon::ipc::IpcServer::bind(&sock).await.unwrap();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let _req = client.read_request().await.unwrap();
        let resp = ctl_daemon::ipc::IpcResponse { diagnostics: "[]".into() };
        client.send_response(&resp).await.unwrap();
    });

    let ready = ctl::daemon::check_ready(&sock).await;
    assert!(ready, "check_ready should return true when server responds");
    handle.await.unwrap();
}

#[tokio::test]
async fn check_ready_nonexistent_socket() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no_such_ready.sock");
    let ready = ctl::daemon::check_ready(&missing).await;
    assert!(!ready, "check_ready should return false for nonexistent socket");
}

#[tokio::test]
async fn nudge_nonexistent_socket_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no_such_socket.sock");
    let result = ctl::daemon::nudge(&missing, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn nudge_with_file_returns_response() {
    let tmp = tempfile::tempdir().unwrap();
    let sock = tmp.path().join("nudge.sock");

    let server = ctl_daemon::ipc::IpcServer::bind(&sock).await.unwrap();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let req = client.read_request().await.unwrap();
        assert_eq!(req.file, Some("src/lib.rs".into()));
        let resp = ctl_daemon::ipc::IpcResponse { diagnostics: "[{}]".into() };
        client.send_response(&resp).await.unwrap();
    });

    let resp = ctl::daemon::nudge(&sock, Some("src/lib.rs")).await.unwrap();
    assert_eq!(resp.diagnostics, "[{}]");
    handle.await.unwrap();
}

#[tokio::test]
async fn nudge_without_file_returns_all() {
    let tmp = tempfile::tempdir().unwrap();
    let sock = tmp.path().join("nudge_all.sock");

    let server = ctl_daemon::ipc::IpcServer::bind(&sock).await.unwrap();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let req = client.read_request().await.unwrap();
        assert!(req.file.is_none());
        let resp = ctl_daemon::ipc::IpcResponse { diagnostics: "all_entries".into() };
        client.send_response(&resp).await.unwrap();
    });

    let resp = ctl::daemon::nudge(&sock, None).await.unwrap();
    assert_eq!(resp.diagnostics, "all_entries");
    handle.await.unwrap();
}
