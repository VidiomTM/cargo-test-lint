use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub file_path: PathBuf,
    pub diagnostics_json: String,
    pub timestamp: u64,
}
