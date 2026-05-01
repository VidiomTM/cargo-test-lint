use std::io::IsTerminal;

pub fn format_summary(finding_count: usize, file_count: usize) -> String {
    if finding_count == 0 {
        format!("\u{2713} 0 findings ({file_count} files checked)")
    } else {
        format!("\u{2717} {finding_count} findings across {file_count} files")
    }
}

pub fn should_print_summary() -> bool {
    std::io::stdout().is_terminal() || std::env::var("RUST_LOG").is_ok()
}
