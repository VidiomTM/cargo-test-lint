use super::Formatter;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::io::Write;

pub struct TerminalFormatter;

impl Formatter for TerminalFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        for diag in diagnostics {
            let level_str = match diag.level {
                DiagnosticLevel::Allow => continue,
                DiagnosticLevel::Warn => "warning",
                DiagnosticLevel::Deny => "error",
                DiagnosticLevel::Forbid => "error",
            };
            let color = match diag.level {
                DiagnosticLevel::Warn => "\x1b[33m",
                DiagnosticLevel::Deny | DiagnosticLevel::Forbid => "\x1b[31m",
                _ => "\x1b[0m",
            };
            let reset = "\x1b[0m";
            let bold = "\x1b[1m";
            let dim = "\x1b[2m";
            writeln!(
                writer,
                "{color}{level_str}{reset}{dim}[{rule}]{reset}: {message}",
                rule = diag.rule_id,
                message = diag.message
            )?;
            writeln!(
                writer,
                "  {bold}-->{reset} {path}:{line}:{col}",
                path = diag.file_path.display(),
                line = diag.line,
                col = diag.column
            )?;
            if let Some(fix) = &diag.suggestion {
                writeln!(
                    writer,
                    "  {dim}|{reset} help: {desc}: `{replacement}`",
                    desc = fix.description,
                    replacement = fix.replacement
                )?;
            }
            writeln!(writer)?;
        }
        let non_allow: Vec<_> =
            diagnostics.iter().filter(|d| d.level != DiagnosticLevel::Allow).collect();
        let errors = non_allow.iter().filter(|d| d.level.is_error()).count();
        let warnings =
            non_allow.iter().filter(|d| matches!(d.level, DiagnosticLevel::Warn)).count();
        if !non_allow.is_empty() {
            let color = if errors > 0 { "\x1b[31m" } else { "\x1b[33m" };
            let reset = "\x1b[0m";
            writeln!(
                writer,
                "{color}{} error{}, {} warning{}{reset}",
                errors,
                if errors == 1 { "" } else { "s" },
                warnings,
                if warnings == 1 { "" } else { "s" }
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use std::path::PathBuf;

    fn make_diag(rule_id: &str, level: DiagnosticLevel, msg: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
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

    fn format(diags: &[Diagnostic]) -> String {
        let formatter = TerminalFormatter;
        let mut buf = Vec::new();
        formatter.write(diags, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn warning_contains_rule_id() {
        let output = format(&[make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "missing msg")]);
        assert!(output.contains("CTL_ASSERT_MSG"), "output should contain rule id");
        assert!(output.contains("warning"), "output should contain 'warning'");
    }

    #[test]
    fn error_contains_file_location() {
        let output = format(&[make_diag("CTL_SLEEP", DiagnosticLevel::Forbid, "sleepy")]);
        assert!(output.contains("src/lib.rs:10:5"), "output should contain file location");
        assert!(output.contains("error"), "output should contain 'error'");
    }

    #[test]
    fn suggestion_rendered() {
        let mut diag = make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "no msg");
        diag.suggestion = Some(Fix {
            description: "add message".into(),
            replacement: "assert!(true, \"msg\")".into(),
            start_byte: 0,
            end_byte: 20,
        });
        let output = format(&[diag]);
        assert!(output.contains("help: add message"), "output should contain suggestion help text");
        assert!(
            output.contains("assert!(true, \"msg\")"),
            "output should contain suggestion replacement"
        );
    }

    #[test]
    fn summary_counts() {
        let diags = vec![
            make_diag("A", DiagnosticLevel::Warn, "w1"),
            make_diag("B", DiagnosticLevel::Warn, "w2"),
            make_diag("C", DiagnosticLevel::Deny, "e1"),
        ];
        let output = format(&diags);
        assert!(output.contains("1 error"), "summary should show 1 error");
        assert!(output.contains("2 warnings"), "summary should show 2 warnings");
    }

    #[test]
    fn empty_diagnostics_no_output() {
        assert!(format(&[]).is_empty(), "empty diagnostics should produce no output");
    }

    #[test]
    fn allow_level_skipped() {
        assert!(
            format(&[make_diag("A", DiagnosticLevel::Allow, "hidden")]).is_empty(),
            "allow-level diagnostics should produce no output"
        );
    }
}
