use crate::diagnostics::Diagnostic;
use std::io::Write;

use super::Formatter;

pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(diagnostics)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::DiagnosticLevel;
    use crate::output::Formatter as _;
    use std::path::PathBuf;

    fn make_diag(rule: &str, level: DiagnosticLevel, msg: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule.into(),
            level,
            message: msg.into(),
            file_path: PathBuf::from("src/lib.rs"),
            line: 10,
            column: 5,
            end_line: 10,
            end_column: 20,
            suggestion: None,
        }
    }

    #[test]
    fn json_output_is_valid_json() {
        let diags = vec![
            make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "assertion missing message"),
            make_diag("CTL_FS_IO", DiagnosticLevel::Deny, "fs::write in test"),
        ];
        let mut buf = Vec::new();
        JsonFormatter.write(&diags, &mut buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed.is_array(), "JSON output should be an array");
        assert_eq!(parsed.as_array().unwrap().len(), 2, "should have 2 diagnostics");
    }

    #[test]
    fn json_empty_diagnostics() {
        let mut buf = Vec::new();
        JsonFormatter.write(&[], &mut buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(parsed.is_array(), "empty output should be an array");
        assert_eq!(parsed.as_array().unwrap().len(), 0, "empty array");
    }

    #[test]
    fn json_contains_rule_id() {
        let diags = vec![make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "msg")];
        let mut buf = Vec::new();
        JsonFormatter.write(&diags, &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("CTL_ASSERT_MSG"), "JSON should contain rule_id");
    }
}
