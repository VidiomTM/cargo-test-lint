use super::Formatter;
use crate::diagnostics::Diagnostic;
use std::io::Write;

pub struct SarifFormatter;

impl Formatter for SarifFormatter {
    fn write(&self, _diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "cargo-test-lint",
                        "version": env!("CARGO_PKG_VERSION"),
                        "rules": []
                    }
                },
                "results": []
            }]
        });
        serde_json::to_writer_pretty(writer, &sarif)?;
        Ok(())
    }
}
