use ctl::output::format_summary;

#[test]
fn summary_zero_findings() {
    assert_eq!(format_summary(0, 5), "\u{2713} 0 findings (5 files checked)");
}

#[test]
fn summary_with_findings() {
    assert_eq!(format_summary(3, 2), "\u{2717} 3 findings across 2 files");
}

#[test]
fn summary_single_finding_single_file() {
    assert_eq!(format_summary(1, 1), "\u{2717} 1 finding across 1 file");
}

#[test]
fn summary_zero_files() {
    assert_eq!(format_summary(0, 0), "\u{2713} 0 findings (0 files checked)");
}

#[test]
fn summary_large_numbers() {
    assert_eq!(format_summary(42, 100), "\u{2717} 42 findings across 100 files");
}

#[test]
fn summary_singular_finding_plural_files() {
    assert_eq!(format_summary(1, 5), "\u{2717} 1 finding across 5 files");
}

#[test]
fn summary_plural_findings_singular_file() {
    assert_eq!(format_summary(3, 1), "\u{2717} 3 findings across 1 file");
}
