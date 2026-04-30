use crate::coverage::CoverageGap;
use crate::mutation::SurvivingMutant;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub file_path: PathBuf,
    pub diagnostics_json: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub level: DiagnosticLevel,
    pub spans: Vec<DiagnosticSpan>,
    pub children: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCode {
    pub code: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub is_primary: bool,
    pub label: Option<String>,
}

pub fn coverage_to_diagnostics(gaps: &[CoverageGap]) -> Vec<Diagnostic> {
    gaps.iter()
        .map(|gap| {
            let kind = if gap.is_branch { "branch" } else { "line" };
            let col_start = gap.column_start.unwrap_or(1) as usize;
            let col_end = gap.column_end.map(|c| c as usize).unwrap_or(col_start + 1);

            Diagnostic {
                message: format!(
                    "uncovered {kind} at line {} (executed {} times)",
                    gap.line, gap.count
                ),
                code: Some(DiagnosticCode {
                    code: "CTL_COVERAGE".into(),
                    explanation: Some("code not covered by tests".into()),
                }),
                level: DiagnosticLevel::Warning,
                spans: vec![DiagnosticSpan {
                    file_name: gap.file_path.clone(),
                    byte_start: 0,
                    byte_end: 0,
                    line_start: gap.line as usize,
                    line_end: gap.line as usize,
                    column_start: col_start,
                    column_end: col_end,
                    is_primary: true,
                    label: Some(format!("uncovered {kind}")),
                }],
                children: vec![],
            }
        })
        .collect()
}

pub fn mutant_to_diagnostics(mutants: &[SurvivingMutant]) -> Vec<Diagnostic> {
    mutants
        .iter()
        .map(|m| {
            let col_start = m.col_start.unwrap_or(1) as usize;
            let col_end = m.col_end.map(|c| c as usize).unwrap_or(col_start + m.replacement.len());

            let mut children: Vec<Diagnostic> = Vec::new();
            if let Some(ref diff) = m.diff_hunk {
                children.push(Diagnostic {
                    message: diff.clone(),
                    code: None,
                    level: DiagnosticLevel::Note,
                    spans: vec![DiagnosticSpan {
                        file_name: m.file_path.clone(),
                        byte_start: 0,
                        byte_end: 0,
                        line_start: m.line as usize,
                        line_end: m.line as usize,
                        column_start: col_start,
                        column_end: col_end,
                        is_primary: false,
                        label: Some("suggested fix".into()),
                    }],
                    children: vec![],
                });
            }

            Diagnostic {
                message: format!(
                    "surviving mutation: {:?} — `{}` → `{}`",
                    m.mutation_type, m.original, m.replacement
                ),
                code: Some(DiagnosticCode {
                    code: "CTL_MUTANT".into(),
                    explanation: Some(
                        "mutation survived — test suite did not kill this mutant".into(),
                    ),
                }),
                level: DiagnosticLevel::Warning,
                spans: vec![DiagnosticSpan {
                    file_name: m.file_path.clone(),
                    byte_start: 0,
                    byte_end: 0,
                    line_start: m.line as usize,
                    line_end: m.line as usize,
                    column_start: col_start,
                    column_end: col_end,
                    is_primary: true,
                    label: Some("surviving mutant".into()),
                }],
                children,
            }
        })
        .collect()
}
