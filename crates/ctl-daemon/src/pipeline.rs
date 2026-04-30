use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use tracing::{error, info, warn};

use crate::cache::Cache;
use crate::cov;
use crate::ipc::{IpcResponse, IpcServer};
use crate::matrix::CoverageMatrix;
use crate::r#mut;
use crate::watch::FileWatcher;

const FULL_SWEEP_INTERVAL: Duration = Duration::from_secs(300);

pub struct Pipeline {
    project_root: PathBuf,
    cache: Cache,
    matrix: Option<CoverageMatrix>,
}

impl Pipeline {
    pub fn new(project_root: PathBuf) -> Self {
        let cache = Cache::new(&project_root);
        Self { project_root, cache, matrix: None }
    }

    pub async fn run_file_scoped(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        for file in changed_files {
            if file.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let rel =
                file.strip_prefix(&self.project_root).unwrap_or(file).to_string_lossy().to_string();

            info!("file-scoped pipeline for {rel}");

            self.cache.invalidate(&[file.clone()]);

            match cov::gaps(&self.project_root).await {
                Ok(gaps) => {
                    let file_matrix = CoverageMatrix::from_gaps(&gaps);
                    match self.matrix.as_mut() {
                        Some(m) => {
                            m.remove_file(file);
                            m.merge(file_matrix);
                        }
                        None => {
                            self.matrix = Some(file_matrix);
                        }
                    }
                }
                Err(e) => {
                    warn!("coverage run failed for {rel}: {e}");
                }
            }

            if let Some(ref matrix) = self.matrix {
                match r#mut::run(&self.project_root, Some(&rel), None).await {
                    Ok(report) => {
                        let filtered = matrix.filter_mutant_targets(&report.mutants);
                        info!(
                            "mutation results for {rel}: {} surviving mutants (filtered)",
                            filtered.len()
                        );

                        let entry = ctl_core::diagnostic::DiagnosticEntry {
                            file_path: file.clone(),
                            diagnostics_json: serde_json::to_string(&filtered)?,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        };
                        self.cache.write_entries(&[entry])?;
                    }
                    Err(e) => {
                        warn!("mutation run failed for {rel}: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn run_full_sweep(&mut self) -> Result<()> {
        info!("starting full sweep for {}", self.project_root.display());

        let gaps = cov::gaps(&self.project_root).await?;
        info!("full coverage: {} gaps found", gaps.len());

        self.matrix = Some(CoverageMatrix::from_gaps(&gaps));

        let report = r#mut::run(&self.project_root, None, None).await?;
        info!(
            "full mutation: {} total, {} survived, {} killed, {} timeout",
            report.total, report.survived, report.killed, report.timeout
        );

        let matrix = self.matrix.as_ref().unwrap();
        let filtered = matrix.filter_mutant_targets(&report.mutants);

        let entries: Vec<ctl_core::diagnostic::DiagnosticEntry> = filtered
            .iter()
            .map(|m| {
                let path = PathBuf::from(&m.file_path);
                ctl_core::diagnostic::DiagnosticEntry {
                    file_path: path,
                    diagnostics_json: serde_json::to_string(&vec![m]).unwrap_or_default(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                }
            })
            .collect();

        self.cache.write_entries(&entries)?;
        info!("full sweep complete, cached {} entries", entries.len());

        Ok(())
    }

    pub async fn serve(&mut self, socket_path: &Path) -> Result<()> {
        let server = IpcServer::bind(socket_path).await?;
        let mut watcher = FileWatcher::new(self.project_root.clone(), 500);
        let mut sweep_interval = tokio::time::interval(FULL_SWEEP_INTERVAL);
        sweep_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        info!("daemon listening on {}", socket_path.display());

        loop {
            tokio::select! {
                changed = watcher.changed_files() => {
                    match changed {
                        Ok(files) => {
                            if let Err(e) = self.run_file_scoped(&files).await {
                                error!("file-scoped pipeline error: {e}");
                            }
                        }
                        Err(e) => {
                            error!("watcher error: {e}");
                        }
                    }
                }

                _ = sweep_interval.tick() => {
                    if let Err(e) = self.run_full_sweep().await {
                        error!("full sweep error: {e}");
                    }
                }

                client = server.accept() => {
                    let mut client = client?;
                    match client.read_request().await {
                        Ok(req) => {
                            let entries = self.cache.read_entries().unwrap_or_default();
                            let filtered: Vec<_> = match req.file {
                                Some(ref f) => {
                                    entries.into_iter().filter(|e| {
                                        e.file_path.to_string_lossy().contains(f.as_str())
                                    }).collect()
                                }
                                None => entries,
                            };
                            let resp = IpcResponse {
                                diagnostics: serde_json::to_string(&filtered)?,
                            };
                            client.send_response(&resp).await?;
                        }
                        Err(e) => {
                            warn!("ipc request error: {e}");
                        }
                    }
                }
            }
        }
    }
}
