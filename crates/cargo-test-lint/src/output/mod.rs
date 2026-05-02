pub mod sarif;
pub mod terminal;

use crate::diagnostics::Diagnostic;
use std::io::Write;

pub trait Formatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()>;
}

pub enum OutputFormat {
    Terminal,
    Sarif,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "terminal" => Ok(Self::Terminal),
            "sarif" => Ok(Self::Sarif),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}
