mod daemon;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use ctl_core::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticEntry, DiagnosticLevel, DiagnosticSpan,
    resolve_byte_offsets,
};
use serde::Serialize;
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "cargo-test-lint", about = "rust-analyzer check.overrideCommand")]
struct Cli {
    #[arg(value_name = "TEST-LINT")]
    _subcommand: Option<String>,

    #[arg(long)]
    file: Option<String>,

    #[arg(long, default_value = ".")]
    project_root: PathBuf,

    #[arg(long)]
    daemon: bool,

    #[arg(last = true)]
    _rest: Vec<String>,
}

#[derive(Serialize)]
struct CargoDiagnostic {
    #[serde(rename = "$message_type")]
    message_type: String,
    code: Option<DiagnosticCode>,
    level: String,
    message: String,
    spans: Vec<DiagnosticSpan>,
    children: Vec<CargoDiagnostic>,
    rendered: String,
}

#[derive(Serialize)]
struct CompilerMessage {
    reason: String,
    package_id: String,
    manifest_path: String,
    target: CargoTarget,
    message: CargoDiagnostic,
}

#[derive(Serialize)]
struct CargoTarget {
    kind: Vec<String>,
    crate_types: Vec<String>,
    name: String,
    src_path: String,
    edition: String,
}

impl CompilerMessage {
    fn from_diagnostic(d: &Diagnostic, project_root: &std::path::Path) -> Self {
        let level = match d.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Note => "note",
        };

        let rendered = format!("{level}: {}", d.message);

        let message = CargoDiagnostic {
            message_type: "diagnostic".into(),
            code: d.code.clone(),
            level: level.into(),
            message: d.message.clone(),
            spans: d.spans.clone(),
            children: d
                .children
                .iter()
                .map(|c| CargoDiagnostic {
                    message_type: "diagnostic".into(),
                    code: c.code.clone(),
                    level: match c.level {
                        DiagnosticLevel::Error => "error".into(),
                        DiagnosticLevel::Warning => "warning".into(),
                        DiagnosticLevel::Note => "note".into(),
                    },
                    message: c.message.clone(),
                    spans: c.spans.clone(),
                    children: vec![],
                    rendered: c.message.clone(),
                })
                .collect(),
            rendered,
        };

        Self {
            reason: "compiler-message".into(),
            package_id: "cargo-test-lint 0.1.0".into(),
            manifest_path: project_root.join("Cargo.toml").to_string_lossy().into(),
            target: CargoTarget {
                kind: vec!["bin".into()],
                crate_types: vec!["bin".into()],
                name: "cargo-test-lint".into(),
                src_path: project_root
                    .join("crates")
                    .join("ctl")
                    .join("src")
                    .join("main.rs")
                    .to_string_lossy()
                    .into(),
                edition: "2021".into(),
            },
            message,
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

    if cli.daemon {
        return run_daemon(&cli.project_root).await;
    }

    let manifest = cli.project_root.join("Cargo.toml");
    if !manifest.exists() {
        anyhow::bail!(
            "no Cargo.toml found in {}. Is this a Rust project?",
            cli.project_root.display()
        );
    }

    let sock = daemon::socket_path(&cli.project_root);

    if !daemon::check_liveness(&sock).await {
        info!("daemon not alive, spawning");
        daemon::spawn_daemon(&cli.project_root).await.map_err(|e| {
            anyhow::anyhow!(
                "failed to spawn daemon: {e}\n  Check logs: {}/target/ctl-daemon.log",
                cli.project_root.display()
            )
        })?;

        let mut alive = false;
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            if daemon::check_liveness(&sock).await {
                alive = true;
                break;
            }
        }
        if !alive {
            anyhow::bail!(
                "daemon did not start within 5s\n\
                 - Check logs: {}/target/ctl-daemon.log\n\
                 - Ensure 'cargo llvm-cov' and 'cargo mutants' are installed\n\
                   cargo install cargo-llvm-cov cargo-mutants",
                cli.project_root.display()
            );
        }
    }

    let resp = daemon::nudge(&sock, cli.file.as_deref()).await.map_err(|e| {
        anyhow::anyhow!(
            "failed to communicate with daemon: {e}\n\
             - Is the daemon running? Check: {}/target/ctl-daemon.sock\n\
             - Try removing stale socket: rm {}/target/ctl-daemon.sock",
            cli.project_root.display(),
            cli.project_root.display()
        )
    })?;

    let entries: Vec<DiagnosticEntry> = match serde_json::from_str(&resp.diagnostics) {
        Ok(e) => e,
        Err(e) => {
            anyhow::bail!(
                "daemon returned invalid JSON — is the project indexed?\n\
                 Parse error: {e}\n\
                 Raw response (first 200 chars): {:.200}",
                resp.diagnostics
            );
        }
    };

    let mut diagnostics: Vec<Diagnostic> = entries
        .iter()
        .flat_map(|e| {
            serde_json::from_str::<Vec<Diagnostic>>(&e.diagnostics_json).unwrap_or_else(|err| {
                warn!(
                    "failed to deserialize diagnostics_json for {}: {err}",
                    e.file_path.display()
                );
                Vec::new()
            })
        })
        .collect();

    let mut sources: HashMap<String, String> = HashMap::new();
    for diag in &diagnostics {
        for span in &diag.spans {
            if !sources.contains_key(&span.file_name) {
                if let Ok(content) = std::fs::read_to_string(&span.file_name) {
                    sources.insert(span.file_name.clone(), content);
                }
            }
        }
    }
    resolve_byte_offsets(&mut diagnostics, &sources);

    for diag in &diagnostics {
        let msg = CompilerMessage::from_diagnostic(diag, &cli.project_root);
        println!("{}", serde_json::to_string(&msg)?);
    }

    if std::env::var("RUST_LOG").is_ok() {
        let file_count = entries.len();
        let finding_count = diagnostics.len();
        if finding_count == 0 {
            eprintln!("✓ 0 findings ({file_count} files checked)");
        } else {
            eprintln!("✗ {finding_count} findings across {file_count} files");
        }
    }

    Ok(())
}

async fn run_daemon(project_root: &std::path::Path) -> Result<()> {
    let mut pipeline = ctl_daemon::pipeline::Pipeline::new(project_root.to_path_buf());
    let sock = daemon::socket_path(project_root);
    pipeline.serve(&sock).await
}
