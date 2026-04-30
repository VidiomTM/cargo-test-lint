use ctl_core::coverage::CoverageGap;
use ctl_core::diagnostic::{DiagnosticLevel, coverage_to_diagnostics, mutant_to_diagnostics};
use ctl_core::mutation::{MutationKind, SurvivingMutant};

#[test]
fn coverage_gap_produces_warning_diagnostic() {
    let gaps = vec![CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 5,
        column_start: Some(8),
        column_end: Some(20),
        count: 0,
        is_branch: true,
    }];

    let diagnostics = coverage_to_diagnostics(&gaps);
    assert_eq!(diagnostics.len(), 1);

    let diag = &diagnostics[0];
    assert!(matches!(diag.level, DiagnosticLevel::Warning));
    assert_eq!(diag.code.as_ref().unwrap().code, "CTL_COVERAGE");
    assert!(diag.message.contains("uncovered branch"));
    assert!(diag.message.contains("line 5"));
    assert_eq!(diag.spans.len(), 1);
    assert!(diag.spans[0].is_primary);
    assert_eq!(diag.spans[0].line_start, 5);
    assert_eq!(diag.spans[0].column_start, 8);
}

#[test]
fn coverage_gap_line_type() {
    let gaps = vec![CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 10,
        column_start: None,
        column_end: None,
        count: 0,
        is_branch: false,
    }];

    let diagnostics = coverage_to_diagnostics(&gaps);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("uncovered line"));
}

#[test]
fn mutant_produces_warning_diagnostic() {
    let mutants = vec![SurvivingMutant {
        file_path: "src/lib.rs".into(),
        line: 3,
        col_start: Some(20),
        col_end: Some(21),
        mutation_type: MutationKind::Arithmetic,
        replacement: "-".into(),
        original: "+".into(),
        diff_hunk: None,
    }];

    let diagnostics = mutant_to_diagnostics(&mutants);
    assert_eq!(diagnostics.len(), 1);

    let diag = &diagnostics[0];
    assert!(matches!(diag.level, DiagnosticLevel::Warning));
    assert_eq!(diag.code.as_ref().unwrap().code, "CTL_MUTANT");
    assert!(diag.message.contains("surviving mutation"));
    assert_eq!(diag.spans.len(), 1);
    assert!(diag.spans[0].is_primary);
    assert!(diag.children.is_empty());
}

#[test]
fn mutant_with_diff_hunk_has_child_diagnostic() {
    let mutants = vec![SurvivingMutant {
        file_path: "src/lib.rs".into(),
        line: 3,
        col_start: None,
        col_end: None,
        mutation_type: MutationKind::ReplaceOperator,
        replacement: "!=".into(),
        original: "==".into(),
        diff_hunk: Some("--- a/src/lib.rs\n+++ b/src/lib.rs".into()),
    }];

    let diagnostics = mutant_to_diagnostics(&mutants);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].children.len(), 1);
    assert!(matches!(diagnostics[0].children[0].level, DiagnosticLevel::Note));
    assert_eq!(diagnostics[0].children[0].spans[0].label.as_deref(), Some("suggested fix"));
}

#[test]
fn empty_inputs_produce_empty_diagnostics() {
    assert!(coverage_to_diagnostics(&[]).is_empty());
    assert!(mutant_to_diagnostics(&[]).is_empty());
}
