use std::io::IsTerminal;

pub fn format_summary(finding_count: usize, file_count: usize) -> String {
    let finding_label = if finding_count == 1 { "finding" } else { "findings" };
    let file_label = if file_count == 1 { "file" } else { "files" };
    if finding_count == 0 {
        format!("\u{2713} 0 findings ({file_count} {file_label} checked)")
    } else {
        format!("\u{2717} {finding_count} {finding_label} across {file_count} {file_label}")
    }
}

pub fn should_print_summary() -> bool {
    std::io::stderr().is_terminal() || std::env::var("RUST_LOG").is_ok()
}
