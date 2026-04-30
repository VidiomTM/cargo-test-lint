use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use tracing::{error, info, warn};

use crate::{
    cache::Cache,
    ipc::{IpcResponse, IpcServer},
    matrix::CoverageMatrix,
    watch::FileWatcher,
};

const FULL_SWEEP_INTERVAL: Duration = Duration::from_secs(300);

pub trait CovRunner: Send + Sync {
    fn gaps(
        &self,
        project_root: &Path,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ctl_core::coverage::CoverageGap>>> + Send>,
    >;
}

pub trait MutRunner: Send + Sync {
    fn run(
        &self,
        project_root: &Path,
        file_filter: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ctl_core::mutation::MutationReport>> + Send>,
    >;
}

pub struct ProductionCovRunner;

impl CovRunner for ProductionCovRunner {
    fn gaps(
        &self,
        project_root: &Path,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<ctl_core::coverage::CoverageGap>>> + Send>,
    > {
        let root = project_root.to_path_buf();
        Box::pin(async move { crate::cov::gaps(&root).await })
    }
}

pub struct ProductionMutRunner;

impl MutRunner for ProductionMutRunner {
    fn run(
        &self,
        project_root: &Path,
        file_filter: Option<&str>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ctl_core::mutation::MutationReport>> + Send>,
    > {
        let root = project_root.to_path_buf();
        let file = file_filter.map(String::from);
        Box::pin(async move { crate::r#mut::run(&root, file.as_deref(), None).await })
    }
}

type BoxedCovRunner = Box<dyn CovRunner>;
type BoxedMutRunner = Box<dyn MutRunner>;

pub struct Pipeline {
    project_root: PathBuf,
    cache: Cache,
    matrix: Option<CoverageMatrix>,
    cov_runner: BoxedCovRunner,
    mut_runner: BoxedMutRunner,
}

impl Pipeline {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root: project_root.clone(),
            cache: Cache::new(&project_root),
            matrix: None,
            cov_runner: Box::new(ProductionCovRunner),
            mut_runner: Box::new(ProductionMutRunner),
        }
    }

    pub fn new_with_runners(
        project_root: PathBuf,
        cov_runner: impl CovRunner + 'static,
        mut_runner: impl MutRunner + 'static,
    ) -> Self {
        Self {
            project_root: project_root.clone(),
            cache: Cache::new(&project_root),
            matrix: None,
            cov_runner: Box::new(cov_runner),
            mut_runner: Box::new(mut_runner),
        }
    }

    pub async fn run_file_scoped(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        for file in changed_files {
            if file.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let rel =
                file.strip_prefix(&self.project_root).unwrap_or(file).to_string_lossy().to_string();

            info!("file-scoped pipeline for {rel}");

            let mut all_diagnostics = Vec::new();

            match self.cov_runner.gaps(&self.project_root).await {
                Ok(gaps) => {
                    let file_gaps: Vec<_> = gaps
                        .iter()
                        .filter(|g| {
                            let gap_path = PathBuf::from(&g.file_path);
                            let stripped = gap_path.strip_prefix(&self.project_root);
                            let rel = stripped.unwrap_or(&gap_path);
                            rel == file.as_path() || file.ends_with(rel)
                        })
                        .cloned()
                        .collect();
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
                    all_diagnostics
                        .extend(ctl_core::diagnostic::coverage_to_diagnostics(&file_gaps));
                }
                Err(e) => {
                    warn!("coverage run failed for {rel}: {e}");
                }
            }

            if let Some(ref matrix) = self.matrix {
                match self.mut_runner.run(&self.project_root, Some(&rel)).await {
                    Ok(report) => {
                        let filtered = matrix.filter_mutant_targets(&report.mutants);
                        info!(
                            "mutation results for {rel}: {} surviving mutants (filtered)",
                            filtered.len()
                        );
                        all_diagnostics
                            .extend(ctl_core::diagnostic::mutant_to_diagnostics(&filtered));
                    }
                    Err(e) => {
                        warn!("mutation run failed for {rel}: {e}");
                    }
                }
            }

            if !all_diagnostics.is_empty() {
                let entry = ctl_core::diagnostic::DiagnosticEntry {
                    file_path: file.clone(),
                    diagnostics_json: serde_json::to_string(&all_diagnostics)?,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                };
                self.cache.upsert_entries(&[entry])?;
            } else {
                self.cache.invalidate(std::slice::from_ref(file));
            }
        }

        Ok(())
    }

    pub async fn run_full_sweep(&mut self) -> Result<()> {
        info!("starting full sweep for {}", self.project_root.display());

        let gaps = self.cov_runner.gaps(&self.project_root).await?;
        info!("full coverage: {} gaps found", gaps.len());

        self.matrix = Some(CoverageMatrix::from_gaps(&gaps));

        let cov_diagnostics = ctl_core::diagnostic::coverage_to_diagnostics(&gaps);

        let report = self.mut_runner.run(&self.project_root, None).await?;
        info!(
            "full mutation: {} total, {} survived, {} killed, {} timeout",
            report.total, report.survived, report.killed, report.timeout
        );

        let matrix = self.matrix.as_ref().unwrap();
        let filtered = matrix.filter_mutant_targets(&report.mutants);
        let mut_diagnostics = ctl_core::diagnostic::mutant_to_diagnostics(&filtered);

        let mut all_diagnostics: Vec<_> = cov_diagnostics;
        all_diagnostics.extend(mut_diagnostics);

        let mut by_file: std::collections::HashMap<PathBuf, Vec<ctl_core::diagnostic::Diagnostic>> =
            std::collections::HashMap::new();
        for diag in &all_diagnostics {
            for span in &diag.spans {
                let path = PathBuf::from(&span.file_name);
                by_file.entry(path).or_default().push(diag.clone());
            }
        }

        let entries: Vec<ctl_core::diagnostic::DiagnosticEntry> = by_file
            .into_iter()
            .map(|(file_path, diags)| ctl_core::diagnostic::DiagnosticEntry {
                file_path,
                diagnostics_json: serde_json::to_string(&diags).unwrap_or_default(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            })
            .collect();

        self.cache.write_entries(&entries)?;
        info!("full sweep complete, cached {} entries", entries.len());

        Ok(())
    }

    pub async fn serve(&mut self, socket_path: &Path) -> Result<()> {
        let server = IpcServer::bind(socket_path).await?;
        info!("daemon listening on {}", socket_path.display());

        info!("warming cache with initial full sweep");
        if let Err(e) = self.run_full_sweep().await {
            warn!("initial sweep failed: {e}");
        }

        let mut watcher = FileWatcher::new(self.project_root.clone(), 500);
        let mut sweep_interval = tokio::time::interval(FULL_SWEEP_INTERVAL);
        sweep_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
