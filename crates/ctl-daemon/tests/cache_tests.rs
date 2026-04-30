use std::path::{Path, PathBuf};

use ctl_core::diagnostic::DiagnosticEntry;
use ctl_daemon::cache::Cache;

fn setup() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn entry(file: &str, json: &str) -> DiagnosticEntry {
    DiagnosticEntry {
        file_path: PathBuf::from(file),
        diagnostics_json: json.to_string(),
        timestamp: 100,
    }
}

#[test]
fn write_and_read_entries() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    let entries = vec![entry("src/a.rs", "[{}]"), entry("src/b.rs", "[{},{}]")];
    cache.write_entries(&entries).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].file_path, PathBuf::from("src/a.rs"));
    assert_eq!(read[1].file_path, PathBuf::from("src/b.rs"));
}

#[test]
fn read_from_empty_dir_returns_empty() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());
    let read = cache.read_entries().unwrap();
    assert!(read.is_empty());
}

#[test]
fn invalidate_removes_matching_entries() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    let entries =
        vec![entry("src/a.rs", "[{}]"), entry("src/b.rs", "[{}]"), entry("src/c.rs", "[{}]")];
    cache.write_entries(&entries).unwrap();
    cache.invalidate(&[PathBuf::from("src/b.rs")]);

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 2);
    assert!(read.iter().all(|e| e.file_path != Path::new("src/b.rs")));
}

#[test]
fn invalidate_noop_for_missing_file() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "[{}]")]).unwrap();
    cache.invalidate(&[PathBuf::from("src/missing.rs")]);

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 1);
}

#[test]
fn upsert_replaces_existing_entry() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "old")]).unwrap();
    cache.upsert_entries(&[entry("src/a.rs", "new")]).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].diagnostics_json, "new");
}

#[test]
fn upsert_adds_new_entry_without_removing_others() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "[{}]")]).unwrap();
    cache.upsert_entries(&[entry("src/b.rs", "[{}]")]).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 2);
}

#[test]
fn upsert_multiple_entries() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "1"), entry("src/b.rs", "2")]).unwrap();
    cache.upsert_entries(&[entry("src/a.rs", "1a"), entry("src/c.rs", "3")]).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 3);

    let a = read.iter().find(|e| e.file_path == Path::new("src/a.rs")).unwrap();
    assert_eq!(a.diagnostics_json, "1a");
}

#[test]
fn upsert_empty_is_noop() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "[{}]")]).unwrap();
    cache.upsert_entries(&[]).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 1);
}

#[test]
fn write_overwrites_existing() {
    let tmp = setup();
    let cache = Cache::new(tmp.path());

    cache.write_entries(&[entry("src/a.rs", "1"), entry("src/b.rs", "2")]).unwrap();
    cache.write_entries(&[entry("src/c.rs", "3")]).unwrap();

    let read = cache.read_entries().unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].file_path, PathBuf::from("src/c.rs"));
}
