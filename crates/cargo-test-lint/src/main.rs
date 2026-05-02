use clap::Parser;
use std::path::PathBuf;

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

    #[arg(long, default_value = "terminal")]
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
    let config = cargo_test_lint::config::load(&args.project_root);
    let files = cargo_test_lint::parser::collect_rs_files(&args.project_root)?;

    let mut all_diagnostics = Vec::new();
    for file in &files {
        let (source, tree) = cargo_test_lint::parser::parse_file(file)?;
        let ctx = cargo_test_lint::rules::RuleContext {
            source: &source,
            tree: &tree,
            config: &config,
            file_path: file,
        };
        all_diagnostics.extend(cargo_test_lint::rules::run_all_rules(&ctx));
    }

    use cargo_test_lint::output::Formatter;
    let formatter: Box<dyn Formatter> = match args.format.as_str() {
        "sarif" => Box::new(cargo_test_lint::output::sarif::SarifFormatter),
        _ => Box::new(cargo_test_lint::output::terminal::TerminalFormatter),
    };

    formatter.write(&all_diagnostics, &mut std::io::stderr())?;

    if cargo_test_lint::diagnostics::Diagnostic::has_errors(&all_diagnostics)
        || (args.deny_warnings && !all_diagnostics.is_empty())
    {
        std::process::exit(1);
    }

    Ok(())
}
