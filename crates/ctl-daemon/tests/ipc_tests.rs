use ctl_daemon::ipc::{IpcRequest, IpcResponse};

#[test]
fn ipc_request_serialization() {
    let req = IpcRequest { file: Some("src/lib.rs".into()) };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("src/lib.rs"));

    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.file, Some("src/lib.rs".into()));
}

#[test]
fn ipc_request_none_file() {
    let req = IpcRequest { file: None };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: IpcRequest = serde_json::from_str(&json).unwrap();
    assert!(parsed.file.is_none());
}

#[test]
fn ipc_response_serialization() {
    let resp = IpcResponse { diagnostics: "[{}]".into() };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: IpcResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.diagnostics, "[{}]");
}

#[tokio::test]
async fn ipc_server_bind_and_connect() {
    let tmp = tempfile::tempdir().unwrap();
    let sock_path = tmp.path().join("test.sock");

    let server = ctl_daemon::ipc::IpcServer::bind(&sock_path).await.unwrap();
    assert!(sock_path.exists());

    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let req = client.read_request().await.unwrap();
        assert_eq!(req.file, Some("src/lib.rs".into()));

        let resp = IpcResponse { diagnostics: "[{}]".into() };
        client.send_response(&resp).await.unwrap();
    });

    let stream = tokio::net::UnixStream::connect(&sock_path).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let req = IpcRequest { file: Some("src/lib.rs".into()) };
    let json = serde_json::to_string(&req).unwrap();
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    writer.write_all(json.as_bytes()).await.unwrap();
    writer.write_u8(b'\n').await.unwrap();
    writer.flush().await.unwrap();

    let mut reader = tokio::io::BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let resp: IpcResponse = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(resp.diagnostics, "[{}]");

    handle.await.unwrap();
}

#[tokio::test]
async fn ipc_bind_creates_parent_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("deep").join("nested").join("test.sock");

    let _server = ctl_daemon::ipc::IpcServer::bind(&nested).await.unwrap();
    assert!(nested.exists());
}

#[tokio::test]
async fn ipc_bind_removes_stale_socket() {
    let tmp = tempfile::tempdir().unwrap();
    let sock_path = tmp.path().join("test.sock");

    std::fs::write(&sock_path, "stale").unwrap();
    assert!(sock_path.exists());

    let _server = ctl_daemon::ipc::IpcServer::bind(&sock_path).await.unwrap();
    assert!(sock_path.exists());
}

#[tokio::test]
async fn ipc_multiple_requests() {
    let tmp = tempfile::tempdir().unwrap();
    let sock_path = tmp.path().join("multi.sock");

    let server = ctl_daemon::ipc::IpcServer::bind(&sock_path).await.unwrap();

    let handle = tokio::spawn(async move {
        for i in 0..3 {
            let mut client = server.accept().await.unwrap();
            let _req = client.read_request().await.unwrap();
            let resp = IpcResponse { diagnostics: format!("resp-{i}") };
            client.send_response(&resp).await.unwrap();
        }
    });

    for i in 0..3 {
        let stream = tokio::net::UnixStream::connect(&sock_path).await.unwrap();
        let (reader, mut writer) = stream.into_split();
        let req = IpcRequest { file: None };
        let json = serde_json::to_string(&req).unwrap();
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        writer.write_all(json.as_bytes()).await.unwrap();
        writer.write_u8(b'\n').await.unwrap();
        writer.flush().await.unwrap();

        let mut reader = tokio::io::BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let resp: IpcResponse = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(resp.diagnostics, format!("resp-{i}"));
    }

    handle.await.unwrap();
}
