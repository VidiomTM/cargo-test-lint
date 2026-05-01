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

pub async fn check_ready(socket_path: &Path) -> bool {
    nudge(socket_path, None).await.is_ok()
}

pub async fn spawn_daemon(project_root: &Path) -> Result<()> {
    let target_dir = project_root.join("target");
    tokio::fs::create_dir_all(&target_dir).await?;
    let log_path = target_dir.join("ctl-daemon.log");
    let log_file = tokio::fs::File::create(&log_path).await?.into_std().await;

    let exe = std::env::current_exe()?;
    let mut cmd = tokio::process::Command::new(exe);
    cmd.args(["--daemon", "--project-root"])
        .arg(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log_file))
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
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        anyhow::bail!("ipc peer closed connection before sending a response");
    }
    let resp: IpcResponse = serde_json::from_str(line.trim())?;
    Ok(resp)
}
