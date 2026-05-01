use ctl_core::coverage::CoverageGap;
use ctl_core::diagnostic::{
    Diagnostic, DiagnosticLevel, DiagnosticSpan, coverage_to_diagnostics, mutant_to_diagnostics,
    resolve_byte_offsets,
};
use ctl_core::mutation::{MutationKind, SurvivingMutant};
use std::collections::HashMap;

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

#[test]
fn resolve_byte_offsets_with_source() {
    let mut diags = vec![Diagnostic {
        message: "test".into(),
        code: None,
        level: DiagnosticLevel::Warning,
        spans: vec![DiagnosticSpan {
            file_name: "test.rs".into(),
            byte_start: 0,
            byte_end: 0,
            line_start: 1,
            line_end: 1,
            column_start: 5,
            column_end: 10,
            is_primary: true,
            label: None,
            suggested_replacement: None,
            suggestion_applicability: None,
            expansion: None,
        }],
        children: vec![],
    }];
    let sources = HashMap::from([("test.rs".into(), "hello world\nsecond line\n".into())]);
    resolve_byte_offsets(&mut diags, &sources);
    assert_ne!(diags[0].spans[0].byte_start, 0);
    assert_ne!(diags[0].spans[0].byte_end, 0);
    assert!(diags[0].spans[0].byte_end > diags[0].spans[0].byte_start);
    let src_len = sources["test.rs"].len();
    assert!(diags[0].spans[0].byte_start < src_len);
    assert!(diags[0].spans[0].byte_end <= src_len);
}

#[test]
fn resolve_byte_offsets_missing_source() {
    let mut diags = vec![Diagnostic {
        message: "test".into(),
        code: None,
        level: DiagnosticLevel::Warning,
        spans: vec![DiagnosticSpan {
            file_name: "missing.rs".into(),
            byte_start: 0,
            byte_end: 0,
            line_start: 1,
            line_end: 1,
            column_start: 1,
            column_end: 1,
            is_primary: true,
            label: None,
            suggested_replacement: None,
            suggestion_applicability: None,
            expansion: None,
        }],
        children: vec![],
    }];
    let sources = HashMap::new();
    resolve_byte_offsets(&mut diags, &sources);
    assert_eq!(diags[0].spans[0].byte_start, 0);
    assert_eq!(diags[0].spans[0].byte_end, 0);
}

#[test]
fn mutant_col_end_fallback_to_original_length() {
    let m = SurvivingMutant {
        file_path: "src/lib.rs".into(),
        line: 3,
        col_start: Some(1),
        col_end: None,
        mutation_type: MutationKind::ReplaceOperator,
        replacement: "-".into(),
        original: "+".into(),
        diff_hunk: None,
    };
    let diags = mutant_to_diagnostics(&[m]);
    assert_eq!(diags[0].spans[0].column_end, 2);
}

#[test]
fn mutant_with_diff_hunk_has_applicability() {
    let m = SurvivingMutant {
        file_path: "src/lib.rs".into(),
        line: 3,
        col_start: Some(1),
        col_end: Some(2),
        mutation_type: MutationKind::ReplaceOperator,
        replacement: "-".into(),
        original: "+".into(),
        diff_hunk: Some("--- a\n+++ b\n".into()),
    };
    let diags = mutant_to_diagnostics(&[m]);
    assert_eq!(diags[0].children.len(), 1);
    assert_eq!(
        diags[0].children[0].spans[0].suggestion_applicability,
        Some("MachineApplicable".into())
    );
}

#[test]
fn coverage_gap_branch_type() {
    let gap = CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 5,
        column_start: Some(1),
        column_end: Some(3),
        count: 0,
        is_branch: true,
    };
    let diags = coverage_to_diagnostics(&[gap]);
    assert_eq!(diags[0].message, "uncovered branch at line 5 (executed 0 times)");
    assert_eq!(diags[0].spans[0].label, Some("uncovered branch".into()));
}

#[test]
fn coverage_gap_no_column_end() {
    let gap = CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 5,
        column_start: None,
        column_end: None,
        count: 0,
        is_branch: false,
    };
    let diags = coverage_to_diagnostics(&[gap]);
    assert_eq!(diags[0].spans[0].column_start, 1);
    assert_eq!(diags[0].spans[0].column_end, 2);
}
