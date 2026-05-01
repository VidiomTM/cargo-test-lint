use ctl_daemon::cov::cov_parse;

#[test]
fn parse_llvm_cov_json_empty_data() {
    let json = r#"{"data":[{"files":[]}]}"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert!(report.files.is_empty());
}

#[test]
fn parse_llvm_cov_json_with_files() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [],
                "summary": {
                    "lines": {"count": 10, "covered": 8, "notcovered": 2, "percent": 80.0}
                }
            }]
        }]
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "src/lib.rs");
    assert_eq!(report.files[0].summary.lines, 10);
    assert_eq!(report.files[0].summary.covered, 8);
}

#[test]
fn parse_llvm_cov_json_invalid_json() {
    let result = cov_parse::parse_llvm_cov_json("not json");
    assert!(result.is_err());
}

#[test]
fn extract_gaps_finds_uncovered_segments() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    {"line": 5, "col": 1, "count": 0, "has_count": true},
                    {"line": 10, "col": 1, "count": 3, "has_count": true},
                    {"line": 15, "col": 5, "count": 0, "has_count": true, "region": 1}
                ]
            }]
        }]
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0].line, 5);
    assert!(!gaps[0].is_branch);
    assert_eq!(gaps[1].line, 15);
    assert!(gaps[1].is_branch);
}

#[test]
fn extract_gaps_skips_segments_without_count() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    {"line": 1, "col": 1, "count": 0, "has_count": false}
                ]
            }]
        }]
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert!(gaps.is_empty());
}

#[test]
fn extract_gaps_multiple_files() {
    let json = r#"{
        "data": [{
            "files": [
                {"filename": "a.rs", "segments": [{"line": 1, "col": 1, "count": 0, "has_count": true}]},
                {"filename": "b.rs", "segments": [{"line": 2, "col": 1, "count": 0, "has_count": true}]}
            ]
        }]
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0].file_path, "a.rs");
    assert_eq!(gaps[1].file_path, "b.rs");
}

#[test]
fn parse_llvm_cov_json_no_summary() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [],
                "summary": null
            }]
        }]
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].summary.lines, 0);
}

#[test]
fn extract_gaps_skips_covered_segments() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    {"line": 5, "col": 1, "count": 10, "has_count": true}
                ]
            }]
        }]
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert!(gaps.is_empty());
}

#[test]
fn parse_llvm_cov_json_aggregates_multiple_data_entries() {
    let json = r#"{
        "data": [
            {"files": [{"filename": "a.rs", "segments": [], "summary": {"lines": {"count": 5, "covered": 3, "notcovered": 2, "percent": 60.0}}}]},
            {"files": [{"filename": "b.rs", "segments": [], "summary": {"lines": {"count": 10, "covered": 10, "notcovered": 0, "percent": 100.0}}}]}
        ]
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 2);
}
