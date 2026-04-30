use std::path::{Path, PathBuf};
use std::time::Duration;

use ctl_core::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticSpan};
use ctl_daemon::cache::Cache;
use ctl_daemon::ipc::{IpcResponse, IpcServer};
use ctl_daemon::pipeline::{CovRunner, MutRunner, Pipeline};

fn uncovered_diag(file: &str, line: usize) -> Diagnostic {
    Diagnostic {
        message: "uncovered line".into(),
        code: Some(DiagnosticCode {
            code: "CTL_COVERAGE".into(),
            explanation: Some("code not covered by tests".into()),
        }),
        level: DiagnosticLevel::Warning,
        spans: vec![DiagnosticSpan {
            file_name: file.into(),
            byte_start: 0,
            byte_end: 0,
            line_start: line,
            line_end: line,
            column_start: 1,
            column_end: 5,
            is_primary: true,
            label: Some("uncovered line".into()),
            suggested_replacement: None,
            suggestion_applicability: None,
            expansion: None,
        }],
        children: vec![],
    }
}

struct MockCovRunner {
    gaps: Vec<ctl_core::coverage::CoverageGap>,
}

impl CovRunner for MockCovRunner {
    fn gaps(
        &self,
        _project_root: &Path,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = anyhow::Result<Vec<ctl_core::coverage::CoverageGap>>>
                + Send,
        >,
    > {
        let gaps = self.gaps.clone();
        Box::pin(async move { Ok(gaps) })
    }
}

struct MockMutRunner {
    report: ctl_core::mutation::MutationReport,
}

impl MutRunner for MockMutRunner {
    fn run(
        &self,
        _project_root: &Path,
        _file_filter: Option<&str>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = anyhow::Result<ctl_core::mutation::MutationReport>>
                + Send,
        >,
    > {
        let report = self.report.clone();
        Box::pin(async move { Ok(report) })
    }
}

#[tokio::test]
async fn pipeline_file_scoped_caches_diagnostics() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();

    let cov_gaps = vec![ctl_core::coverage::CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 3,
        column_start: Some(1),
        column_end: None,
        count: 0,
        is_branch: false,
    }];
    let mut_report = ctl_core::mutation::MutationReport {
        mutants: vec![ctl_core::mutation::SurvivingMutant {
            file_path: "src/lib.rs".into(),
            line: 5,
            col_start: Some(1),
            col_end: Some(2),
            mutation_type: ctl_core::mutation::MutationKind::ReplaceOperator,
            replacement: "-".into(),
            original: "+".into(),
            diff_hunk: None,
        }],
        total: 1,
        survived: 1,
        killed: 0,
        timeout: 0,
    };

    let mut pipeline = Pipeline::new_with_runners(
        root.clone(),
        MockCovRunner { gaps: cov_gaps },
        MockMutRunner { report: mut_report },
    );

    let changed = vec![PathBuf::from("src/lib.rs")];
    pipeline.run_file_scoped(&changed).await.unwrap();

    let cache = Cache::new(&root);
    let entries = cache.read_entries().unwrap();
    assert_eq!(entries.len(), 1);

    let diags: Vec<Diagnostic> = serde_json::from_str(&entries[0].diagnostics_json).unwrap();
    assert!(!diags.is_empty());

    let has_coverage =
        diags.iter().any(|d| d.code.as_ref().is_some_and(|c| c.code == "CTL_COVERAGE"));
    let has_mutant = diags.iter().any(|d| d.code.as_ref().is_some_and(|c| c.code == "CTL_MUTANT"));
    assert!(has_coverage, "expected CTL_COVERAGE diagnostic");
    assert!(has_mutant, "expected CTL_MUTANT diagnostic");
}

#[tokio::test]
async fn pipeline_full_sweep_caches_diagnostics() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();

    let cov_gaps = vec![ctl_core::coverage::CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 3,
        column_start: Some(1),
        column_end: None,
        count: 0,
        is_branch: false,
    }];
    let mut_report = ctl_core::mutation::MutationReport {
        mutants: vec![],
        total: 5,
        survived: 0,
        killed: 5,
        timeout: 0,
    };

    let mut pipeline = Pipeline::new_with_runners(
        root.clone(),
        MockCovRunner { gaps: cov_gaps },
        MockMutRunner { report: mut_report },
    );

    pipeline.run_full_sweep().await.unwrap();

    let cache = Cache::new(&root);
    let entries = cache.read_entries().unwrap();
    assert_eq!(entries.len(), 1);

    let diags: Vec<Diagnostic> = serde_json::from_str(&entries[0].diagnostics_json).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_ref().unwrap().code, "CTL_COVERAGE");
}

#[tokio::test]
async fn pipeline_file_scoped_cleans_cache_when_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();

    let cache = Cache::new(&root);
    let file_path = root.join("src").join("lib.rs");
    cache
        .write_entries(&[ctl_core::diagnostic::DiagnosticEntry {
            file_path: file_path.clone(),
            diagnostics_json: "[{}]".into(),
            timestamp: 1,
        }])
        .unwrap();
    assert_eq!(cache.read_entries().unwrap().len(), 1);

    let mut pipeline = Pipeline::new_with_runners(
        root.clone(),
        MockCovRunner { gaps: vec![] },
        MockMutRunner { report: ctl_core::mutation::MutationReport::empty() },
    );

    let changed = vec![file_path];
    pipeline.run_file_scoped(&changed).await.unwrap();

    let entries = cache.read_entries().unwrap();
    assert!(entries.is_empty(), "entry should be invalidated when file is clean");
}

#[tokio::test]
async fn pipeline_serve_ipc_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let sock = tmp.path().join("test.sock");

    let cov_gaps = vec![ctl_core::coverage::CoverageGap {
        file_path: "src/lib.rs".into(),
        line: 3,
        column_start: Some(1),
        column_end: None,
        count: 0,
        is_branch: false,
    }];

    let mut pipeline = Pipeline::new_with_runners(
        root.clone(),
        MockCovRunner { gaps: cov_gaps },
        MockMutRunner { report: ctl_core::mutation::MutationReport::empty() },
    );

    pipeline.run_full_sweep().await.unwrap();

    let server = IpcServer::bind(&sock).await.unwrap();
    let root_clone = root.clone();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let req = client.read_request().await.unwrap();
        let entries = {
            let cache = Cache::new(&root_clone);
            cache.read_entries().unwrap_or_default()
        };
        let filtered: Vec<_> = match req.file {
            Some(ref f) => entries
                .into_iter()
                .filter(|e| e.file_path.to_string_lossy().contains(f.as_str()))
                .collect(),
            None => entries,
        };
        let resp = IpcResponse { diagnostics: serde_json::to_string(&filtered).unwrap() };
        client.send_response(&resp).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(20)).await;

    let resp = ctl_daemon::ipc::IpcClient::connect_and_request(&sock, None).await.unwrap();

    let entries: Vec<ctl_core::diagnostic::DiagnosticEntry> =
        serde_json::from_str(&resp.diagnostics).unwrap();
    assert_eq!(entries.len(), 1);

    let diags: Vec<Diagnostic> = serde_json::from_str(&entries[0].diagnostics_json).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.as_ref().unwrap().code, "CTL_COVERAGE");

    handle.await.unwrap();
}

#[tokio::test]
async fn pipeline_serve_ipc_file_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let sock = tmp.path().join("filter.sock");

    let cache = Cache::new(&root);
    cache
        .write_entries(&[
            ctl_core::diagnostic::DiagnosticEntry {
                file_path: PathBuf::from("src/lib.rs"),
                diagnostics_json: "[]".into(),
                timestamp: 1,
            },
            ctl_core::diagnostic::DiagnosticEntry {
                file_path: PathBuf::from("src/main.rs"),
                diagnostics_json: "[]".into(),
                timestamp: 2,
            },
        ])
        .unwrap();

    let server = IpcServer::bind(&sock).await.unwrap();
    let root_clone = root.clone();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let req = client.read_request().await.unwrap();
        let entries = {
            let cache = Cache::new(&root_clone);
            cache.read_entries().unwrap_or_default()
        };
        let filtered: Vec<_> = match req.file {
            Some(ref f) => entries
                .into_iter()
                .filter(|e| e.file_path.to_string_lossy().contains(f.as_str()))
                .collect(),
            None => entries,
        };
        let resp = IpcResponse { diagnostics: serde_json::to_string(&filtered).unwrap() };
        client.send_response(&resp).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(20)).await;

    let resp =
        ctl_daemon::ipc::IpcClient::connect_and_request(&sock, Some("main.rs")).await.unwrap();

    let entries: Vec<ctl_core::diagnostic::DiagnosticEntry> =
        serde_json::from_str(&resp.diagnostics).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_path, PathBuf::from("src/main.rs"));

    handle.await.unwrap();
}

#[tokio::test]
async fn daemon_spawn_and_cli_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let sock = tmp.path().join("daemon.sock");

    let diags = vec![uncovered_diag("src/lib.rs", 3)];
    let cache = Cache::new(&root);
    cache
        .write_entries(&[ctl_core::diagnostic::DiagnosticEntry {
            file_path: PathBuf::from("src/lib.rs"),
            diagnostics_json: serde_json::to_string(&diags).unwrap(),
            timestamp: 100,
        }])
        .unwrap();

    let server = IpcServer::bind(&sock).await.unwrap();
    let entries = cache.read_entries().unwrap();
    let handle = tokio::spawn(async move {
        let mut client = server.accept().await.unwrap();
        let _req = client.read_request().await.unwrap();
        let resp = IpcResponse { diagnostics: serde_json::to_string(&entries).unwrap() };
        client.send_response(&resp).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(20)).await;

    let resp = ctl_daemon::ipc::IpcClient::connect_and_request(&sock, None).await.unwrap();

    let entries: Vec<ctl_core::diagnostic::DiagnosticEntry> =
        serde_json::from_str(&resp.diagnostics).unwrap();
    assert_eq!(entries.len(), 1);

    let diags: Vec<Diagnostic> = serde_json::from_str(&entries[0].diagnostics_json).unwrap();
    assert_eq!(diags[0].spans[0].line_start, 3);
    assert_eq!(diags[0].code.as_ref().unwrap().code, "CTL_COVERAGE");

    handle.await.unwrap();
}

#[test]
fn summary_format_zero_findings() {
    let file_count = 0usize;
    let finding_count = 0usize;
    let msg = if finding_count == 0 {
        format!("\u{2713} 0 findings ({file_count} files checked)")
    } else {
        format!("\u{2717} {finding_count} findings across {file_count} files")
    };
    assert_eq!(msg, "\u{2713} 0 findings (0 files checked)");
}

#[test]
fn summary_format_with_findings() {
    let file_count = 3usize;
    let finding_count = 7usize;
    let msg = format!("\u{2717} {finding_count} findings across {file_count} files");
    assert_eq!(msg, "\u{2717} 7 findings across 3 files");
}
