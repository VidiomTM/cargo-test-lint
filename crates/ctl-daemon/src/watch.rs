use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::warn;

pub struct FileWatcher {
    project_root: PathBuf,
    debounce: Duration,
    rx: mpsc::Receiver<Result<Vec<PathBuf>>>,
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(project_root: PathBuf, debounce_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel(256);
        let debounce = Duration::from_millis(debounce_ms);

        let tx_clone = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let tx = tx_clone.clone();
                tokio::spawn(async move {
                    let files = match res {
                        Ok(event) => event
                            .paths
                            .into_iter()
                            .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
                            .collect::<Vec<_>>(),
                        Err(e) => {
                            warn!("watch error: {e}");
                            Vec::new()
                        }
                    };
                    if !files.is_empty() {
                        let _ = tx.send(Ok(files)).await;
                    }
                });
            },
            Config::default(),
        )
        .expect("failed to create file watcher");

        watcher.watch(&project_root, RecursiveMode::Recursive).expect("failed to start watching");

        Self { project_root, debounce, rx, _watcher: watcher }
    }

    pub async fn changed_files(&mut self) -> Result<Vec<PathBuf>> {
        let mut accumulated: Vec<PathBuf> =
            self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("watcher channel closed"))??;

        let deadline = tokio::time::Instant::now() + self.debounce;
        loop {
            tokio::select! {
                () = tokio::time::sleep_until(deadline) => break,
                res = self.rx.recv() => {
                    match res {
                        Some(Ok(files)) => accumulated.extend(files),
                        Some(Err(e)) => warn!("watch error: {e}"),
                        None => break,
                    }
                }
            }
        }

        let gitignore = Arc::new(
            ignore::gitignore::GitignoreBuilder::new(&self.project_root).build().unwrap_or_else(
                |_| ignore::gitignore::GitignoreBuilder::new(&self.project_root).build().unwrap(),
            ),
        );

        let mut seen = HashSet::new();
        let filtered: Vec<PathBuf> = accumulated
            .into_iter()
            .filter(|p| {
                if seen.contains(p) {
                    return false;
                }
                seen.insert(p.clone());
                let rel = p.strip_prefix(&self.project_root).unwrap_or(p);
                gitignore.matched(rel, false).is_none()
            })
            .collect();

        Ok(filtered)
    }
}
