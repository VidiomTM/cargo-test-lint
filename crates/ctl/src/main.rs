mod daemon;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use ctl_core::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticSpan};
use serde::Serialize;
use tracing::debug;

#[derive(Parser)]
#[command(name = "cargo-test-lint", about = "rust-analyzer check.overrideCommand")]
struct Cli {
    #[arg(long)]
    file: Option<String>,

    #[arg(long, default_value = ".")]
    project_root: PathBuf,
}

#[derive(Serialize)]
struct CompilerMessage {
    reason: String,
    model_id: String,
    code: Option<DiagnosticCode>,
    message: String,
    level: String,
    spans: Vec<DiagnosticSpan>,
    children: Vec<CompilerMessage>,
}

impl CompilerMessage {
    fn from_diagnostic(d: &Diagnostic) -> Self {
        let level = match d.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
        };
        Self {
            reason: "compiler-message".into(),
            model_id: "cargo-test-lint".into(),
            code: d.code.clone(),
            message: d.message.clone(),
            level: level.into(),
            spans: d.spans.clone(),
            children: d.children.iter().map(Self::from_diagnostic).collect(),
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    if let Err(e) = run().await {
        tracing::error!("fatal: {e:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let sock = daemon::socket_path(&cli.project_root);

    if !daemon::check_liveness(&sock).await {
        debug!("daemon not alive, spawning");
        daemon::spawn_daemon(&cli.project_root).await?;

        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if daemon::check_liveness(&sock).await {
                break;
            }
        }
    }

    let resp = daemon::nudge(&sock, cli.file.as_deref()).await?;

    let diagnostics: Vec<Diagnostic> = serde_json::from_str(&resp.diagnostics).unwrap_or_default();

    for diag in &diagnostics {
        let msg = CompilerMessage::from_diagnostic(diag);
        println!("{}", serde_json::to_string(&msg)?);
    }

    Ok(())
}
