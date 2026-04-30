use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcRequest {
    pub file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub diagnostics: String,
}

#[cfg(unix)]
pub struct IpcServer {
    listener: tokio::net::UnixListener,
}

#[cfg(unix)]
impl IpcServer {
    pub async fn bind(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        let listener = tokio::net::UnixListener::bind(path)?;
        Ok(Self { listener })
    }

    pub async fn accept(&self) -> Result<IpcClient> {
        let (stream, _addr) = self.listener.accept().await?;
        Ok(IpcClient { stream })
    }
}

#[cfg(unix)]
pub struct IpcClient {
    stream: tokio::net::UnixStream,
}

#[cfg(unix)]
impl IpcClient {
    pub async fn connect_and_request(
        socket_path: &Path,
        file: Option<&str>,
    ) -> Result<IpcResponse> {
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

    pub async fn read_request(&mut self) -> Result<IpcRequest> {
        let (reader, _) = self.stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let req: IpcRequest = serde_json::from_str(line.trim())?;
        Ok(req)
    }

    pub async fn send_response(&mut self, resp: &IpcResponse) -> Result<()> {
        let (_, writer) = self.stream.split();
        let mut writer = writer;
        let json = serde_json::to_string(resp)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_u8(b'\n').await?;
        writer.flush().await?;
        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<IpcResponse> {
        let (reader, _) = self.stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let resp: IpcResponse = serde_json::from_str(line.trim())?;
        Ok(resp)
    }
}

#[cfg(windows)]
pub struct IpcServer {
    pipe_path: String,
}

#[cfg(windows)]
impl IpcServer {
    pub async fn bind(path: &Path) -> Result<Self> {
        let pipe_path = format!(r"\\.\pipe\{}", path.display());
        Ok(Self { pipe_path })
    }

    pub async fn accept(&self) -> Result<IpcClient> {
        use tokio::net::windows::named_pipe::ServerOptions;
        let server = ServerOptions::new().create(&self.pipe_path)?;
        server.connect().await?;
        Ok(IpcClient { stream: server })
    }
}

#[cfg(windows)]
pub struct IpcClient {
    stream: tokio::net::windows::named_pipe::NamedPipeServer,
}

#[cfg(windows)]
impl IpcClient {
    pub async fn read_request(&mut self) -> Result<IpcRequest> {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 4096];
        let n = self.stream.read(&mut buf).await?;
        let req: IpcRequest = serde_json::from_slice(&buf[..n])?;
        Ok(req)
    }

    pub async fn send_response(&mut self, resp: &IpcResponse) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let json = serde_json::to_string(resp)?;
        self.stream.write_all(json.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;
        Ok(())
    }
}
