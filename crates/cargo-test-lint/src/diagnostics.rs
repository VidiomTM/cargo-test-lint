use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Allow,
    Warn,
    #[serde(alias = "error")]
    Deny,
    Forbid,
}

impl DiagnosticLevel {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Deny | Self::Forbid)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        }
    }
}

impl std::fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DiagnosticLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "allow" => Ok(Self::Allow),
            "warn" => Ok(Self::Warn),
            "deny" | "error" => Ok(Self::Deny),
            "forbid" => Ok(Self::Forbid),
            _ => Err(format!("invalid level: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub level: DiagnosticLevel,
    pub message: String,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub suggestion: Option<Fix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub description: String,
    pub replacement: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl Diagnostic {
    pub fn has_errors(diagnostics: &[Self]) -> bool {
        diagnostics.iter().any(|d| d.level.is_error())
    }

    pub fn sort_by_position(diagnostics: &mut [Self]) {
        diagnostics.sort_by(|a, b| {
            a.file_path.cmp(&b.file_path).then(a.line.cmp(&b.line)).then(a.column.cmp(&b.column))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_level_from_str_valid() {
        assert_eq!(
            "allow".parse::<DiagnosticLevel>().unwrap(),
            DiagnosticLevel::Allow,
            "'allow' should parse to Allow"
        );
        assert_eq!(
            "warn".parse::<DiagnosticLevel>().unwrap(),
            DiagnosticLevel::Warn,
            "'warn' should parse to Warn"
        );
        assert_eq!(
            "deny".parse::<DiagnosticLevel>().unwrap(),
            DiagnosticLevel::Deny,
            "'deny' should parse to Deny"
        );
        assert_eq!(
            "error".parse::<DiagnosticLevel>().unwrap(),
            DiagnosticLevel::Deny,
            "'error' should parse to Deny"
        );
        assert_eq!(
            "forbid".parse::<DiagnosticLevel>().unwrap(),
            DiagnosticLevel::Forbid,
            "'forbid' should parse to Forbid"
        );
    }

    #[test]
    fn diagnostic_level_from_str_invalid() {
        assert!(
            "invalid".parse::<DiagnosticLevel>().is_err(),
            "invalid level string should fail to parse"
        );
    }

    #[test]
    fn diagnostic_level_is_error() {
        assert!(!DiagnosticLevel::Allow.is_error(), "Allow should not be an error level");
        assert!(!DiagnosticLevel::Warn.is_error(), "Warn should not be an error level");
        assert!(DiagnosticLevel::Deny.is_error(), "Deny should be an error level");
        assert!(DiagnosticLevel::Forbid.is_error(), "Forbid should be an error level");
    }

    #[test]
    fn has_errors_detects_violations() {
        let clean: Vec<Diagnostic> = vec![];
        assert!(!Diagnostic::has_errors(&clean), "empty diagnostics should have no errors");

        let warnings = vec![make_diag(DiagnosticLevel::Warn)];
        assert!(!Diagnostic::has_errors(&warnings), "warn-level diagnostics should have no errors");

        let errors = vec![make_diag(DiagnosticLevel::Deny)];
        assert!(Diagnostic::has_errors(&errors), "deny-level diagnostics should have errors");
    }

    #[test]
    fn sort_by_position_same_file_same_line() {
        let mut diags = vec![make_diag_at("a.rs", 5, 3), make_diag_at("a.rs", 5, 1)];
        Diagnostic::sort_by_position(&mut diags);
        assert_eq!(diags[0].file_path, PathBuf::from("a.rs"), "first diag should be in a.rs");
        assert_eq!(diags[0].line, 5, "first diag should be on line 5");
        assert_eq!(diags[0].column, 1, "first diag should be at column 1");
        assert_eq!(diags[1].file_path, PathBuf::from("a.rs"), "second diag should be in a.rs");
        assert_eq!(diags[1].column, 3, "second diag should be at column 3");
    }

    #[test]
    fn sort_by_position_different_files() {
        let mut diags = vec![make_diag_at("b.rs", 10, 1), make_diag_at("a.rs", 5, 1)];
        Diagnostic::sort_by_position(&mut diags);
        assert_eq!(diags[0].file_path, PathBuf::from("a.rs"), "a.rs should sort before b.rs");
        assert_eq!(diags[1].file_path, PathBuf::from("b.rs"), "third diag should be in b.rs");
    }

    fn make_diag(level: DiagnosticLevel) -> Diagnostic {
        let mut d = make_diag_at("test.rs", 1, 1);
        d.level = level;
        d
    }

    fn make_diag_at(path: &str, line: usize, col: usize) -> Diagnostic {
        Diagnostic {
            rule_id: "CTL_TEST".into(),
            level: DiagnosticLevel::Warn,
            message: "test".into(),
            file_path: PathBuf::from(path),
            line,
            column: col,
            end_line: line,
            end_column: col + 5,
            suggestion: None,
        }
    }
}
