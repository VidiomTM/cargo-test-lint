use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use ctl_core::coverage::CoverageGap;
use ctl_core::mutation::SurvivingMutant;
use tracing::info;

pub struct FileCoverage {
    pub lines: HashSet<u32>,
    pub branches: HashMap<u32, bool>,
}

pub struct CoverageMatrix {
    inner: HashMap<PathBuf, FileCoverage>,
}

impl CoverageMatrix {
    pub fn from_gaps(gaps: &[CoverageGap]) -> Self {
        let mut inner: HashMap<PathBuf, FileCoverage> = HashMap::new();

        for gap in gaps {
            let path = PathBuf::from(&gap.file_path);
            let entry = inner.entry(path).or_insert_with(|| FileCoverage {
                lines: HashSet::new(),
                branches: HashMap::new(),
            });

            if gap.is_branch {
                entry.branches.insert(gap.line, false);
            }
            entry.lines.insert(gap.line);
        }

        Self { inner }
    }

    pub fn is_covered(&self, file: &Path, line: u32) -> bool {
        match self.inner.get(file) {
            Some(fc) => !fc.lines.contains(&line),
            None => true,
        }
    }

    pub fn uncovered_lines(&self, file: &Path) -> Vec<u32> {
        match self.inner.get(file) {
            Some(fc) => {
                let mut lines: Vec<u32> = fc.lines.iter().copied().collect();
                lines.sort_unstable();
                lines
            }
            None => Vec::new(),
        }
    }

    pub fn filter_mutant_targets(&self, mutants: &[SurvivingMutant]) -> Vec<SurvivingMutant> {
        let before = mutants.len();
        let filtered: Vec<SurvivingMutant> = mutants
            .iter()
            .filter(|m| {
                let path = PathBuf::from(&m.file_path);
                self.is_covered(&path, m.line)
            })
            .cloned()
            .collect();

        let skipped = before - filtered.len();
        if skipped > 0 {
            info!("filtered out {skipped} mutants on uncovered lines (kept {})", filtered.len());
        }

        filtered
    }

    pub fn merge(&mut self, other: CoverageMatrix) {
        for (path, fc) in other.inner {
            let entry = self.inner.entry(path).or_insert_with(|| FileCoverage {
                lines: HashSet::new(),
                branches: HashMap::new(),
            });
            entry.lines.extend(fc.lines);
            entry.branches.extend(fc.branches);
        }
    }

    pub fn remove_file(&mut self, file: &Path) {
        self.inner.remove(file);
    }
}
