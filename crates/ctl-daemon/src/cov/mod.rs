pub mod cov_parse;

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, bail};
use ctl_core::coverage::{CoverageGap, CoverageReport};
use tokio::process::Command;

pub use cov_parse::{extract_gaps, parse_llvm_cov_json};

const COV_TIMEOUT: Duration = Duration::from_secs(300);

pub async fn run(project_root: &Path) -> anyhow::Result<CoverageReport> {
    let output = tokio::time::timeout(
        COV_TIMEOUT,
        Command::new("cargo").args(["llvm-cov", "--json"]).current_dir(project_root).output(),
    )
    .await
    .context("cargo llvm-cov timed out after 5 minutes")?
    .context("failed to spawn `cargo llvm-cov`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cargo llvm-cov failed (exit {:?}): {}", output.status.code(), stderr.trim());
    }

    let stdout =
        String::from_utf8(output.stdout).context("cargo llvm-cov produced non-UTF-8 output")?;

    let parsed =
        cov_parse::parse_llvm_cov_json(&stdout).context("failed to parse llvm-cov JSON")?;

    Ok(parsed)
}

pub async fn gaps(project_root: &Path) -> anyhow::Result<Vec<CoverageGap>> {
    let output = tokio::time::timeout(
        COV_TIMEOUT,
        Command::new("cargo").args(["llvm-cov", "--json"]).current_dir(project_root).output(),
    )
    .await
    .context("cargo llvm-cov timed out after 5 minutes")?
    .context("failed to spawn `cargo llvm-cov`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cargo llvm-cov failed (exit {:?}): {}", output.status.code(), stderr.trim());
    }

    let stdout =
        String::from_utf8(output.stdout).context("cargo llvm-cov produced non-UTF-8 output")?;

    extract_gaps(&stdout)
}
