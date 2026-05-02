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
    fn diagnostic_level_from_str() {
        assert_eq!("allow".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Allow);
        assert_eq!("warn".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Warn);
        assert_eq!("deny".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Deny);
        assert_eq!("error".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Deny);
        assert_eq!("forbid".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Forbid);
        assert!("invalid".parse::<DiagnosticLevel>().is_err());
    }

    #[test]
    fn diagnostic_level_is_error() {
        assert!(!DiagnosticLevel::Allow.is_error());
        assert!(!DiagnosticLevel::Warn.is_error());
        assert!(DiagnosticLevel::Deny.is_error());
        assert!(DiagnosticLevel::Forbid.is_error());
    }

    #[test]
    fn has_errors_detects_violations() {
        let clean: Vec<Diagnostic> = vec![];
        assert!(!Diagnostic::has_errors(&clean));

        let warnings = vec![make_diag(DiagnosticLevel::Warn)];
        assert!(!Diagnostic::has_errors(&warnings));

        let errors = vec![make_diag(DiagnosticLevel::Deny)];
        assert!(Diagnostic::has_errors(&errors));
    }

    #[test]
    fn sort_by_position_orders_correctly() {
        let mut diags = vec![
            make_diag_at("b.rs", 10, 1),
            make_diag_at("a.rs", 5, 3),
            make_diag_at("a.rs", 5, 1),
        ];
        Diagnostic::sort_by_position(&mut diags);
        assert_eq!(diags[0].file_path, PathBuf::from("a.rs"));
        assert_eq!(diags[0].line, 5);
        assert_eq!(diags[0].column, 1);
        assert_eq!(diags[1].file_path, PathBuf::from("a.rs"));
        assert_eq!(diags[1].line, 5);
        assert_eq!(diags[1].column, 3);
        assert_eq!(diags[2].file_path, PathBuf::from("b.rs"));
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
