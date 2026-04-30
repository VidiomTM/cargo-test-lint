use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Result;
use ctl_daemon::ipc::{IpcRequest, IpcResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub fn socket_path(project_root: &Path) -> PathBuf {
    project_root.join("target").join("ctl-daemon.sock")
}

pub async fn check_liveness(socket_path: &Path) -> bool {
    tokio::net::UnixStream::connect(socket_path).await.is_ok()
}

pub async fn spawn_daemon(project_root: &Path) -> Result<()> {
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(["test-lint-daemon", "--project-root"])
        .arg(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);

    cmd.spawn()?;
    Ok(())
}

pub async fn nudge(socket_path: &Path, file: Option<&str>) -> Result<IpcResponse> {
    let stream = tokio::net::UnixStream::connect(socket_path).await?;
    let (reader, mut writer) = stream.into_split();

    let req = IpcRequest { file: file.map(String::from) };
    let json = serde_json::to_string(&req)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_u8(b'\n').await?;
    writer.flush().await?;

    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let resp: IpcResponse = serde_json::from_str(line.trim())?;
    Ok(resp)
}
