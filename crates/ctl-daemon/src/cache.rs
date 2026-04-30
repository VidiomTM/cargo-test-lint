use std::{
    collections::HashSet,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use ctl_core::diagnostic::DiagnosticEntry;

pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new(project_root: &Path) -> Self {
        let cache_dir = project_root.join("target").join("ctl-cache");
        Self { cache_dir }
    }

    pub fn upsert_entries(&self, new_entries: &[DiagnosticEntry]) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;

        let mut existing = self.read_entries().unwrap_or_default();

        let new_paths: HashSet<PathBuf> = new_entries.iter().map(|e| e.file_path.clone()).collect();
        existing.retain(|e| !new_paths.contains(&e.file_path));
        existing.extend(new_entries.iter().cloned());

        self.write_entries(&existing)
    }

    pub fn write_entries(&self, entries: &[DiagnosticEntry]) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;

        let tmp_path = self.cache_dir.join("diagnostics.ndjson.tmp");
        let final_path = self.cache_dir.join("diagnostics.ndjson");

        {
            let mut file = fs::File::create(&tmp_path)?;
            for entry in entries {
                let line = serde_json::to_string(entry)?;
                writeln!(file, "{line}")?;
            }
            file.flush()?;
        }

        fs::rename(&tmp_path, &final_path)?;
        Ok(())
    }

    pub fn read_entries(&self) -> Result<Vec<DiagnosticEntry>> {
        let path = self.cache_dir.join("diagnostics.ndjson");
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: DiagnosticEntry = serde_json::from_str(trimmed)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn invalidate(&self, files: &[PathBuf]) {
        let to_remove: HashSet<PathBuf> = files.iter().cloned().collect();

        let Ok(mut entries) = self.read_entries() else {
            return;
        };

        let before = entries.len();
        entries.retain(|e| !to_remove.contains(&e.file_path));

        if entries.len() != before {
            if let Err(e) = self.write_entries(&entries) {
                tracing::warn!("cache invalidate write failed: {e}");
            }
        }
    }
}
