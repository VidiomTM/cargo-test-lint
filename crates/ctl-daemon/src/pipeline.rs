use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{Mutex, Semaphore};
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

type ArcCovRunner = Arc<dyn CovRunner>;
type ArcMutRunner = Arc<dyn MutRunner>;

pub struct Pipeline {
    project_root: PathBuf,
    cache: Arc<Cache>,
    matrix: Arc<Mutex<Option<CoverageMatrix>>>,
    cov_runner: ArcCovRunner,
    mut_runner: ArcMutRunner,
}

impl Pipeline {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            cache: Arc::new(Cache::new(&project_root)),
            project_root: project_root.clone(),
            matrix: Arc::new(Mutex::new(None)),
            cov_runner: Arc::new(ProductionCovRunner),
            mut_runner: Arc::new(ProductionMutRunner),
        }
    }

    pub fn new_with_runners(
        project_root: PathBuf,
        cov_runner: impl CovRunner + 'static,
        mut_runner: impl MutRunner + 'static,
    ) -> Self {
        Self {
            cache: Arc::new(Cache::new(&project_root)),
            project_root: project_root.clone(),
            matrix: Arc::new(Mutex::new(None)),
            cov_runner: Arc::new(cov_runner),
            mut_runner: Arc::new(mut_runner),
        }
    }

    pub fn set_cov_runner(&mut self, runner: impl CovRunner + 'static) {
        self.cov_runner = Arc::new(runner);
    }

    pub fn set_mut_runner(&mut self, runner: impl MutRunner + 'static) {
        self.mut_runner = Arc::new(runner);
    }

    pub async fn run_file_scoped(&self, changed_files: &[PathBuf]) -> Result<()> {
        for file in changed_files {
            if file.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let rel =
                file.strip_prefix(&self.project_root).unwrap_or(file).to_string_lossy().to_string();

            info!("file-scoped pipeline for {rel}");

            let mut all_diagnostics = Vec::new();

            let gaps = match self.cov_runner.gaps(&self.project_root).await {
                Ok(gaps) => gaps,
                Err(e) => {
                    warn!("coverage run failed for {rel}: {e}");
                    self.cache.invalidate(std::slice::from_ref(file));
                    continue;
                }
            };

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

            all_diagnostics.extend(ctl_core::diagnostic::coverage_to_diagnostics(&file_gaps));

            match self.mut_runner.run(&self.project_root, Some(&rel)).await {
                Ok(report) => {
                    let filtered = file_matrix.filter_mutant_targets(&report.mutants);
                    info!(
                        "mutation results for {rel}: {} surviving mutants (filtered)",
                        filtered.len()
                    );
                    all_diagnostics.extend(ctl_core::diagnostic::mutant_to_diagnostics(&filtered));
                }
                Err(e) => {
                    warn!("mutation run failed for {rel}: {e}");
                }
            }

            {
                let mut matrix = self.matrix.lock().await;
                *matrix = Some(file_matrix);
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

    pub async fn run_full_sweep(&self) -> Result<()> {
        info!("starting full sweep for {}", self.project_root.display());

        let gaps = self.cov_runner.gaps(&self.project_root).await?;
        info!("full coverage: {} gaps found", gaps.len());

        let cov_diagnostics = ctl_core::diagnostic::coverage_to_diagnostics(&gaps);

        let matrix = CoverageMatrix::from_gaps(&gaps);

        let report = self.mut_runner.run(&self.project_root, None).await?;
        info!(
            "full mutation: {} total, {} survived, {} killed, {} timeout",
            report.total, report.survived, report.killed, report.timeout
        );

        let filtered = matrix.filter_mutant_targets(&report.mutants);

        {
            let mut shared_matrix = self.matrix.lock().await;
            *shared_matrix = Some(matrix);
        }
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

    pub async fn serve(self, socket_path: &Path) -> Result<()> {
        let server = IpcServer::bind(socket_path).await?;
        info!("daemon listening on {}", socket_path.display());

        let cache = Arc::clone(&self.cache);
        let pipeline = Arc::new(self);
        let run_gate = Arc::new(Semaphore::new(1));

        let pipeline_clone = Arc::clone(&pipeline);
        let gate = Arc::clone(&run_gate);
        tokio::spawn(async move {
            let Ok(_permit) = gate.acquire_owned().await else {
                return;
            };
            info!("warming cache with initial full sweep");
            if let Err(e) = pipeline_clone.run_full_sweep().await {
                warn!("initial sweep failed: {e}");
            }
        });

        let mut watcher = FileWatcher::new(pipeline.project_root.clone(), 500);
        let mut sweep_interval = tokio::time::interval(FULL_SWEEP_INTERVAL);
        sweep_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                changed = watcher.changed_files() => {
                    let pipeline = Arc::clone(&pipeline);
                    let gate = Arc::clone(&run_gate);
                    tokio::spawn(async move {
                        let Ok(_permit) = gate.acquire_owned().await else { return; };
                        match changed {
                            Ok(files) => {
                                if let Err(e) = pipeline.run_file_scoped(&files).await {
                                    error!("file-scoped pipeline error: {e}");
                                }
                            }
                            Err(e) => {
                                error!("watcher error: {e}");
                            }
                        }
                    });
                }

                _ = sweep_interval.tick() => {
                    let pipeline = Arc::clone(&pipeline);
                    let gate = Arc::clone(&run_gate);
                    tokio::spawn(async move {
                        let Ok(_permit) = gate.acquire_owned().await else { return; };
                        if let Err(e) = pipeline.run_full_sweep().await {
                            error!("full sweep error: {e}");
                        }
                    });
                }

                client = server.accept() => {
                    let cache = Arc::clone(&cache);
                    let mut client = client?;
                    match client.read_request().await {
                        Ok(req) => {
                            let entries = cache.read_entries().unwrap_or_default();
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

impl Clone for Pipeline {
    fn clone(&self) -> Self {
        Self {
            project_root: self.project_root.clone(),
            cache: Arc::clone(&self.cache),
            matrix: Arc::clone(&self.matrix),
            cov_runner: Arc::clone(&self.cov_runner),
            mut_runner: Arc::clone(&self.mut_runner),
        }
    }
}
