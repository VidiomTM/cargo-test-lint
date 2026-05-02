use super::Formatter;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use serde_json::json;
use std::io::Write;

pub struct SarifFormatter;

impl Formatter for SarifFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        let rules: Vec<serde_json::Value> = diagnostics.iter()
            .map(|d| d.rule_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|id| json!({ "id": id, "name": id, "shortDescription": { "text": id }, "defaultConfiguration": { "level": "warning" } }))
            .collect();

        let results: Vec<serde_json::Value> = diagnostics.iter().map(|d| {
            let level = match d.level {
                DiagnosticLevel::Allow => "none",
                DiagnosticLevel::Warn => "warning",
                DiagnosticLevel::Deny | DiagnosticLevel::Forbid => "error",
            };
            let mut result = json!({
                "ruleId": d.rule_id, "level": level,
                "message": { "text": d.message },
                "locations": [{ "physicalLocation": {
                    "artifactLocation": { "uri": d.file_path.display().to_string() },
                    "region": { "startLine": d.line, "startColumn": d.column, "endLine": d.end_line, "endColumn": d.end_column }
                }}]
            });
            if let Some(fix) = &d.suggestion {
                result["fixes"] = json!([{
                    "description": { "text": &fix.description },
                    "artifactChanges": [{
                        "artifactLocation": { "uri": d.file_path.display().to_string() },
                        "replacements": [{
                            "deletedRegion": { "byteOffset": fix.start_byte, "byteLength": fix.end_byte - fix.start_byte },
                            "insertedContent": { "text": &fix.replacement }
                        }]
                    }]
                }]);
            }
            result
        }).collect();

        let sarif = json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{ "tool": { "driver": {
                "name": "cargo-test-lint",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/user/cargo-test-lint",
                "rules": rules
            }}, "results": results }]
        });
        serde_json::to_writer_pretty(writer, &sarif)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use std::path::PathBuf;

    fn make_diag(rule_id: &str, level: DiagnosticLevel) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
            level,
            message: "test message".into(),
            file_path: PathBuf::from("src/lib.rs"),
            line: 10,
            column: 5,
            end_line: 10,
            end_column: 20,
            suggestion: None,
        }
    }

    fn format(diags: &[Diagnostic]) -> serde_json::Value {
        let formatter = SarifFormatter;
        let mut buf = Vec::new();
        formatter.write(diags, &mut buf).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    #[test]
    fn sarif_version() {
        assert_eq!(format(&[])["version"], "2.1.0");
    }

    #[test]
    fn sarif_has_tool_driver() {
        assert_eq!(format(&[])["runs"][0]["tool"]["driver"]["name"], "cargo-test-lint");
    }

    #[test]
    fn sarif_contains_results() {
        let sarif = format(&[make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn)]);
        assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
        assert_eq!(sarif["runs"][0]["results"][0]["ruleId"], "CTL_ASSERT_MSG");
    }

    #[test]
    fn sarif_level_mapping() {
        let sarif =
            format(&[make_diag("A", DiagnosticLevel::Warn), make_diag("B", DiagnosticLevel::Deny)]);
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "warning");
        assert_eq!(sarif["runs"][0]["results"][1]["level"], "error");
    }

    #[test]
    fn sarif_has_rules() {
        let sarif = format(&[make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn)]);
        let rules = sarif["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["id"], "CTL_ASSERT_MSG");
    }

    #[test]
    fn sarif_includes_fix() {
        let mut diag = make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn);
        diag.suggestion = Some(Fix {
            description: "add message".into(),
            replacement: "assert!(true, \"msg\")".into(),
            start_byte: 0,
            end_byte: 15,
        });
        let sarif = format(&[diag]);
        assert!(sarif["runs"][0]["results"][0]["fixes"].is_array());
    }

    #[test]
    fn sarif_valid_json() {
        let formatter = SarifFormatter;
        let mut buf = Vec::new();
        formatter.write(&[make_diag("A", DiagnosticLevel::Warn)], &mut buf).unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&buf).is_ok());
    }
}
