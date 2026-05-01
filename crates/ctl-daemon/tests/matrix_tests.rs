use std::path::Path;

use ctl_core::coverage::CoverageGap;
use ctl_core::mutation::SurvivingMutant;
use ctl_daemon::matrix::CoverageMatrix;

fn gap(file: &str, line: u32, is_branch: bool) -> CoverageGap {
    CoverageGap {
        file_path: file.to_string(),
        line,
        column_start: Some(1),
        column_end: None,
        count: 0,
        is_branch,
    }
}

fn mutant(file: &str, line: u32) -> SurvivingMutant {
    SurvivingMutant {
        file_path: file.to_string(),
        line,
        col_start: Some(1),
        col_end: Some(10),
        mutation_type: ctl_core::mutation::MutationKind::ReplaceOperator,
        replacement: "-".into(),
        original: "+".into(),
        diff_hunk: None,
    }
}

#[test]
fn from_gaps_creates_matrix() {
    let gaps = vec![gap("a.rs", 5, false), gap("a.rs", 10, false), gap("b.rs", 3, false)];
    let matrix = CoverageMatrix::from_gaps(&gaps);

    assert!(!matrix.is_covered(Path::new("a.rs"), 5));
    assert!(!matrix.is_covered(Path::new("a.rs"), 10));
    assert!(!matrix.is_covered(Path::new("b.rs"), 3));
    assert!(matrix.is_covered(Path::new("a.rs"), 1));
    assert!(matrix.is_covered(Path::new("c.rs"), 1));
}

#[test]
fn is_covered_returns_true_for_unknown_file() {
    let matrix = CoverageMatrix::from_gaps(&[]);
    assert!(matrix.is_covered(Path::new("unknown.rs"), 1));
}

#[test]
fn uncovered_lines_returns_sorted_lines() {
    let gaps = vec![gap("a.rs", 20, false), gap("a.rs", 5, false), gap("a.rs", 15, false)];
    let matrix = CoverageMatrix::from_gaps(&gaps);

    let lines = matrix.uncovered_lines(Path::new("a.rs"));
    assert_eq!(lines, vec![5, 15, 20]);
}

#[test]
fn uncovered_lines_empty_for_unknown_file() {
    let matrix = CoverageMatrix::from_gaps(&[]);
    assert!(matrix.uncovered_lines(Path::new("unknown.rs")).is_empty());
}

#[test]
fn filter_mutant_targets_keeps_covered_mutants() {
    let gaps = vec![gap("a.rs", 5, false)];
    let matrix = CoverageMatrix::from_gaps(&gaps);

    let mutants = vec![mutant("a.rs", 5), mutant("a.rs", 10), mutant("b.rs", 1)];
    let filtered = matrix.filter_mutant_targets(&mutants);

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].line, 10);
    assert_eq!(filtered[1].file_path, "b.rs");
}

#[test]
fn filter_mutant_targets_all_pass_when_no_coverage() {
    let matrix = CoverageMatrix::from_gaps(&[]);
    let mutants = vec![mutant("a.rs", 1), mutant("b.rs", 2)];
    assert_eq!(matrix.filter_mutant_targets(&mutants).len(), 2);
}

#[test]
fn merge_combines_two_matrices() {
    let m1 = CoverageMatrix::from_gaps(&[gap("a.rs", 5, false)]);
    let m2 = CoverageMatrix::from_gaps(&[gap("b.rs", 10, false)]);

    let mut merged = m1;
    merged.merge(m2);

    assert!(!merged.is_covered(Path::new("a.rs"), 5));
    assert!(!merged.is_covered(Path::new("b.rs"), 10));
}

#[test]
fn remove_file_removes_entry() {
    let matrix = CoverageMatrix::from_gaps(&[gap("a.rs", 5, false), gap("b.rs", 3, false)]);
    let mut m = matrix;
    m.remove_file(Path::new("a.rs"));

    assert!(m.is_covered(Path::new("a.rs"), 5));
    assert!(!m.is_covered(Path::new("b.rs"), 3));
}

#[test]
fn branch_gap_marks_line_uncovered() {
    let gaps = vec![gap("a.rs", 5, true)];
    let matrix = CoverageMatrix::from_gaps(&gaps);
    assert!(!matrix.is_covered(Path::new("a.rs"), 5));
}

#[test]
fn merge_overlapping_files_combines_lines() {
    let m1 = CoverageMatrix::from_gaps(&[gap("a.rs", 5, false)]);
    let m2 = CoverageMatrix::from_gaps(&[gap("a.rs", 10, false)]);

    let mut merged = m1;
    merged.merge(m2);

    assert!(!merged.is_covered(Path::new("a.rs"), 5));
    assert!(!merged.is_covered(Path::new("a.rs"), 10));
    assert!(merged.is_covered(Path::new("a.rs"), 7));
}

#[test]
fn filter_mutant_targets_empty_mutants() {
    let matrix = CoverageMatrix::from_gaps(&[gap("a.rs", 5, false)]);
    let filtered = matrix.filter_mutant_targets(&[]);
    assert!(filtered.is_empty());
}
