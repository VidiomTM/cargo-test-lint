use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageGap {
    pub file_path: String,
    pub line: u32,
    pub column_start: Option<u32>,
    pub column_end: Option<u32>,
    pub count: u64,
    pub is_branch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub lines: u64,
    pub covered: u64,
    pub not_covered: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageFile {
    pub path: String,
    pub summary: CoverageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub files: Vec<CoverageFile>,
    pub generated_at: String,
}
