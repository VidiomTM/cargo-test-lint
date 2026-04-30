use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::process::Command;
use tracing::{debug, info, warn};

use ctl_core::mutation::MutationReport;

use crate::mut_parse;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

pub async fn run(
    project_root: &Path,
    file_filter: Option<&str>,
    line_filter: Option<Vec<u32>>,
) -> Result<MutationReport> {
    let mut cmd = Command::new("cargo");
    cmd.arg("mutants").arg("--output-format").arg("json").arg("--output").arg("mutants.out");

    if let Some(file) = file_filter {
        cmd.arg("--file").arg(file);
    }

    if let Some(ref lines) = line_filter {
        for line in lines {
            cmd.arg("--line").arg(line.to_string());
        }
    }

    cmd.current_dir(project_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    debug!("spawning: {:?}", cmd);

    let output = tokio::time::timeout(DEFAULT_TIMEOUT, cmd.output())
        .await
        .context("cargo mutants timed out after 10 minutes")
        .and_then(|r| r.context("failed to spawn cargo mutants"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("cargo mutants exited with {}: {}", output.status, stderr);
    }

    let mutants_dir = project_root.join("mutants.out");
    if !mutants_dir.exists() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "mutants.out directory not found after cargo mutants run\nstdout: {stdout}\nstderr: {stderr}"
        );
    }

    let report = mut_parse::parse_outcomes_from_dir(&mutants_dir)?;
    info!(
        "mutation report: {} total, {} survived, {} killed, {} timeout",
        report.total, report.survived, report.killed, report.timeout
    );

    Ok(report)
}
