use super::Formatter;
use crate::diagnostics::Diagnostic;
use std::io::Write;

pub struct TerminalFormatter;

impl Formatter for TerminalFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        for diag in diagnostics {
            writeln!(writer, "{}", diag.message)?;
        }
        Ok(())
    }
}
