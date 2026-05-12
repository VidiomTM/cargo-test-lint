use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

use cargo_test_lint::config;
use cargo_test_lint::diagnostics::Diagnostic;
use cargo_test_lint::output::{Formatter, OutputFormat};
use cargo_test_lint::parser;
use cargo_test_lint::rules;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    #[command(name = "test-lint")]
    TestLint(TestLintArgs),
}

#[derive(Parser)]
struct TestLintArgs {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,

    #[arg(long)]
    fix: bool,

    #[arg(long)]
    rules: Option<String>,

    #[arg(long, default_value = "terminal", value_name = "FORMAT")]
    /// Output format: terminal, json, sarif
    format: String,

    #[arg(long)]
    max_expects: Option<usize>,

    #[arg(long)]
    nextest: bool,

    #[arg(long)]
    deny_warnings: bool,
}

fn main() -> anyhow::Result<()> {
    let Cargo::TestLint(args) = Cargo::parse();

    let mut config = config::load(&args.project_root);

    if let Some(max) = args.max_expects {
        config.max_expects = max;
    }
    if args.nextest {
        config.nextest = true;
    }
    if args.deny_warnings {
        config.deny_warnings = true;
    }

    let files =
        parser::collect_rs_files(&args.project_root).context("failed to collect source files")?;

    let mut all_diagnostics = Vec::new();

    for file in &files {
        let (source, tree) = match parser::parse_file(file) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("warning: skipping {}: {}", file.display(), e);
                continue;
            }
        };

        let ctx =
            rules::RuleContext { source: &source, tree: &tree, config: &config, file_path: file };

        all_diagnostics.extend(rules::run_all_rules(&ctx));
    }

    let format: OutputFormat = args.format.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    let formatter: Box<dyn Formatter> = match format {
        OutputFormat::Terminal => Box::new(cargo_test_lint::output::terminal::TerminalFormatter),
        OutputFormat::Json => Box::new(cargo_test_lint::output::json::JsonFormatter),
        OutputFormat::Sarif => Box::new(cargo_test_lint::output::sarif::SarifFormatter),
    };

    formatter.write(&all_diagnostics, &mut std::io::stderr())?;

    if Diagnostic::has_errors(&all_diagnostics)
        || (config.deny_warnings && !all_diagnostics.is_empty())
    {
        std::process::exit(1);
    }

    Ok(())
}
