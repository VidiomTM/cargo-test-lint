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
                    "lines": {"count": 10, "covered": 8, "notcovered": 2, "percent": 80.0},
                    "regions": {"count": 20, "covered": 15, "notcovered": 5, "percent": 75.0},
                    "branches": {"count": 0, "covered": 0, "notcovered": 0, "percent": 0.0},
                    "functions": {"count": 5, "covered": 4, "percent": 80.0},
                    "instantiations": {"count": 8, "covered": 6, "percent": 75.0}
                }
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "src/lib.rs");
    assert_eq!(report.files[0].summary.lines, 10);
    assert_eq!(report.files[0].summary.covered, 8);
}

#[test]
fn parse_llvm_cov_json_real_format() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/main.rs",
                "segments": [
                    [1, 1, 4, true, true, false],
                    [5, 10, 0, true, true, false],
                    [10, 1, 3, true, false, false]
                ],
                "summary": {
                    "lines": {"count": 10, "covered": 8, "percent": 80.0},
                    "regions": {"count": 5, "covered": 3, "notcovered": 2, "percent": 60.0},
                    "branches": {"count": 0, "covered": 0, "notcovered": 0, "percent": 0.0},
                    "functions": {"count": 2, "covered": 1, "percent": 50.0},
                    "instantiations": {"count": 3, "covered": 2, "percent": 66.7}
                }
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1",
        "cargo_llvm_cov": "0.1.0"
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].path, "src/main.rs");
    assert_eq!(report.files[0].summary.lines, 10);
    assert_eq!(report.files[0].summary.covered, 8);
    assert_eq!(report.files[0].summary.not_covered, 2);
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
                    [5, 1, 0, true, true, false],
                    [10, 1, 3, true, true, false],
                    [15, 5, 0, true, true, true]
                ]
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert_eq!(gaps.len(), 1, "should skip gap segment (is_gap=true)");
    assert_eq!(gaps[0].line, 5);
    assert!(gaps[0].is_branch, "is_region_entry=true maps to is_branch");
}

#[test]
fn extract_gaps_finds_uncovered_with_region_entry() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    [15, 5, 0, true, true, false]
                ]
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].is_branch, "is_region_entry=true means is_branch=true");
}

#[test]
fn extract_gaps_skips_segments_without_count() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    [1, 1, 0, false, true, false]
                ]
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert!(gaps.is_empty(), "has_count=false segments should be skipped");
}

#[test]
fn extract_gaps_multiple_files() {
    let json = r#"{
        "data": [{
            "files": [
                {"filename": "a.rs", "segments": [[1, 1, 0, true, true, false]], "summary": null},
                {"filename": "b.rs", "segments": [[2, 1, 0, true, true, false]], "summary": null}
            ],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
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
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
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
                    [5, 1, 10, true, true, false]
                ]
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert!(gaps.is_empty());
}

#[test]
fn parse_llvm_cov_json_aggregates_multiple_data_entries() {
    let json = r#"{
        "data": [
            {
                "files": [{"filename": "a.rs", "segments": [], "summary": {"lines": {"count": 5, "covered": 3, "notcovered": 2, "percent": 60.0}, "regions": {"count": 4, "covered": 2, "notcovered": 2, "percent": 50.0}, "branches": {"count": 0, "covered": 0, "percent": 0.0}, "functions": {"count": 1, "covered": 1, "percent": 100.0}, "instantiations": {"count": 1, "covered": 1, "percent": 100.0}}}],
                "functions": [],
                "totals": {}
            },
            {
                "files": [{"filename": "b.rs", "segments": [], "summary": {"lines": {"count": 10, "covered": 10, "notcovered": 0, "percent": 100.0}, "regions": {"count": 8, "covered": 8, "notcovered": 0, "percent": 100.0}, "branches": {"count": 0, "covered": 0, "percent": 0.0}, "functions": {"count": 2, "covered": 2, "percent": 100.0}, "instantiations": {"count": 2, "covered": 2, "percent": 100.0}}}],
                "functions": [],
                "totals": {}
            }
        ],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files.len(), 2);
}

#[test]
fn extract_gaps_skips_gap_segments() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [
                    [5, 1, 0, true, true, true],
                    [10, 1, 0, true, true, false]
                ]
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let gaps = cov_parse::extract_gaps(json).unwrap();
    assert_eq!(gaps.len(), 1, "is_gap=true should be skipped");
    assert_eq!(gaps[0].line, 10);
}

#[test]
fn parse_llvm_cov_json_lines_without_notcovered() {
    let json = r#"{
        "data": [{
            "files": [{
                "filename": "src/lib.rs",
                "segments": [],
                "summary": {
                    "lines": {"count": 100, "covered": 90, "percent": 90.0},
                    "regions": {"count": 50, "covered": 40, "notcovered": 10, "percent": 80.0},
                    "branches": {"count": 0, "covered": 0, "percent": 0.0},
                    "functions": {"count": 5, "covered": 5, "percent": 100.0},
                    "instantiations": {"count": 5, "covered": 5, "percent": 100.0}
                }
            }],
            "functions": [],
            "totals": {}
        }],
        "type": "llvm.coverage.json.export",
        "version": "2.0.1"
    }"#;
    let report = cov_parse::parse_llvm_cov_json(json).unwrap();
    assert_eq!(report.files[0].summary.lines, 100);
    assert_eq!(report.files[0].summary.covered, 90);
    assert_eq!(report.files[0].summary.not_covered, 10, "notcovered inferred from count-covered");
}
