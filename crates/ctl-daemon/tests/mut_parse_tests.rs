use ctl_daemon::mut_parse;

fn outcomes_json_with_surviving() -> &'static str {
    r#"{
        "outcomes": [
            {"scenario": "baseline", "summary": "success", "phase_results": []},
            {
                "scenario": {"mutant": {
                    "name": "replace_arithmetic_operator",
                    "package": "my_pkg",
                    "file": "src/lib.rs",
                    "function": {"function_name": "add", "return_type": "i32", "span": {"start": {"line": 1, "column": 1}, "end": {"line": 3, "column": 2}}},
                    "span": {"start": {"line": 2, "column": 5}, "end": {"line": 2, "column": 6}},
                    "replacement": "-",
                    "genre": "binary_operator"
                }},
                "summary": "missed_mutant",
                "diff_path": null,
                "phase_results": []
            }
        ],
        "total_mutants": 2,
        "missed": 1,
        "caught": 1,
        "timeout": 0
    }"#
}

#[test]
fn parse_outcomes_finds_surviving_mutant() {
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(outcomes_json_with_surviving(), tmp.path()).unwrap();

    assert_eq!(report.total, 2);
    assert_eq!(report.survived, 1);
    assert_eq!(report.killed, 1);
    assert_eq!(report.timeout, 0);
    assert_eq!(report.mutants.len(), 1);

    let m = &report.mutants[0];
    assert_eq!(m.file_path, "src/lib.rs");
    assert_eq!(m.line, 2);
    assert_eq!(m.replacement, "-");
}

#[test]
fn parse_outcomes_empty() {
    let json = r#"{
        "outcomes": [],
        "total_mutants": 0,
        "missed": 0,
        "caught": 0,
        "timeout": 0
    }"#;
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert_eq!(report.total, 0);
    assert!(report.mutants.is_empty());
}

#[test]
fn parse_outcomes_invalid_json() {
    let tmp = tempfile::tempdir().unwrap();
    let result = mut_parse::parse_outcomes("not json", tmp.path());
    assert!(result.is_err());
}

#[test]
fn parse_outcomes_skips_baseline() {
    let json = r#"{
        "outcomes": [
            {"scenario": "baseline", "summary": "success", "phase_results": []}
        ],
        "total_mutants": 0,
        "missed": 0,
        "caught": 0,
        "timeout": 0
    }"#;
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert!(report.mutants.is_empty());
}

#[test]
fn parse_outcomes_skips_caught_mutant() {
    let json = r#"{
        "outcomes": [
            {
                "scenario": {"mutant": {
                    "name": "replace_arithmetic_operator",
                    "package": "pkg",
                    "file": "src/lib.rs",
                    "span": {"start": {"line": 1, "column": 1}, "end": {"line": 1, "column": 2}},
                    "replacement": "-",
                    "genre": "binary_operator"
                }},
                "summary": "caught_mutant",
                "phase_results": []
            }
        ],
        "total_mutants": 1,
        "missed": 0,
        "caught": 1,
        "timeout": 0
    }"#;
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert!(report.mutants.is_empty());
    assert_eq!(report.killed, 1);
}

#[test]
fn parse_outcomes_reads_diff_hunk() {
    let tmp = tempfile::tempdir().unwrap();
    let diff_content = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-  a + b\n+  a - b\n";
    std::fs::write(tmp.path().join("diff.patch"), diff_content).unwrap();

    let json = r#"{
        "outcomes": [
            {
                "scenario": {"mutant": {
                    "name": "replace_arithmetic_operator",
                    "package": "pkg",
                    "file": "src/lib.rs",
                    "span": {"start": {"line": 1, "column": 1}, "end": {"line": 1, "column": 2}},
                    "replacement": "-",
                    "genre": "binary_operator"
                }},
                "summary": "missed_mutant",
                "diff_path": "diff.patch",
                "phase_results": []
            }
        ],
        "total_mutants": 1,
        "missed": 1,
        "caught": 0,
        "timeout": 0
    }"#;
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert_eq!(report.mutants.len(), 1);
    assert!(report.mutants[0].diff_hunk.is_some());
    assert!(report.mutants[0].diff_hunk.as_ref().unwrap().contains("a - b"));
}

#[test]
fn parse_outcomes_from_dir() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("outcomes.json"), outcomes_json_with_surviving()).unwrap();

    let report = mut_parse::parse_outcomes_from_dir(tmp.path()).unwrap();
    assert_eq!(report.mutants.len(), 1);
}

#[test]
fn parse_outcomes_from_dir_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let result = mut_parse::parse_outcomes_from_dir(tmp.path());
    assert!(result.is_err());
}

#[test]
fn parse_outcomes_fn_value_mutation_kind() {
    let json = r#"{
        "outcomes": [
            {
                "scenario": {"mutant": {
                    "name": "void_return_value",
                    "package": "pkg",
                    "file": "src/lib.rs",
                    "function": {"function_name": "foo", "return_type": "i32", "span": {"start": {"line": 1, "column": 1}, "end": {"line": 3, "column": 2}}},
                    "span": {"start": {"line": 2, "column": 5}, "end": {"line": 2, "column": 6}},
                    "replacement": "()",
                    "genre": "fn_value"
                }},
                "summary": "missed_mutant",
                "phase_results": []
            }
        ],
        "total_mutants": 1,
        "missed": 1,
        "caught": 0,
        "timeout": 0
    }"#;
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert_eq!(report.mutants[0].mutation_type, ctl_core::mutation::MutationKind::VoidReturnValue);
    assert_eq!(report.mutants[0].original, "fn foo() { ... }");
}

#[test]
fn parse_outcomes_multiple_surviving() {
    let json = r#"{
        "outcomes": [
            {
                "scenario": {"mutant": {
                    "name": "replace_arithmetic_operator",
                    "package": "pkg",
                    "file": "a.rs",
                    "span": {"start": {"line": 1, "column": 1}, "end": {"line": 1, "column": 2}},
                    "replacement": "-",
                    "genre": "binary_operator"
                }},
                "summary": "missed_mutant",
                "phase_results": []
            },
            {
                "scenario": {"mutant": {
                    "name": "remove_statement",
                    "package": "pkg",
                    "file": "b.rs",
                    "span": {"start": {"line": 5, "column": 1}, "end": {"line": 5, "column": 10}},
                    "replacement": "",
                    "genre": "match_arm"
                }},
                "summary": "missed_mutant",
                "phase_results": []
            }
        ],
        "total_mutants": 2,
        "missed": 2,
        "caught": 0,
        "timeout": 0
    }"#;
    let tmp = tempfile::tempdir().unwrap();
    let report = mut_parse::parse_outcomes(json, tmp.path()).unwrap();
    assert_eq!(report.mutants.len(), 2);
}
