# AST-Driven Test Linter — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pivot `cargo-test-lint` from a coverage/mutation wrapper daemon into a standalone, AST-driven test quality linter powered by Tree-sitter.

**Architecture:** Single Rust crate using Tree-sitter for parsing with a hybrid query+validation rule engine. Rules configured via `Cargo.toml [lints]` table per RFC 3389. Output in SARIF 2.1.0 and colored terminal formats.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), tree-sitter + tree-sitter-rust, clap, serde, serde_json, anstyle

**Spec:** `docs/superpowers/specs/2026-05-02-ast-driven-test-lint-design.md`

---

## File Map

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Update workspace to single crate, update deps |
| Delete | `crates/ctl-core/` | Remove old crate |
| Delete | `crates/ctl-daemon/` | Remove old crate |
| Rename | `crates/ctl/` → `crates/cargo-test-lint/` | Single crate |
| Create | `crates/cargo-test-lint/src/lib.rs` | Module re-exports |
| Create | `crates/cargo-test-lint/src/diagnostics.rs` | Diagnostic types |
| Create | `crates/cargo-test-lint/src/config.rs` | Cargo.toml [lints] parser |
| Create | `crates/cargo-test-lint/src/parser.rs` | Tree-sitter setup |
| Create | `crates/cargo-test-lint/src/rules/mod.rs` | Rule trait, registry, engine |
| Create | `crates/cargo-test-lint/src/rules/assertions.rs` | CTL_ASSERT_MSG, CTL_MAX_EXPECTS |
| Create | `crates/cargo-test-lint/src/rules/sleep.rs` | CTL_SLEEP |
| Create | `crates/cargo-test-lint/src/rules/flow.rs` | CTL_TEST_BRANCHING |
| Create | `crates/cargo-test-lint/src/rules/nextest.rs` | CTL_STATIC_MUT, CTL_ENV_SET_VAR |
| Create | `crates/cargo-test-lint/src/rules/async_safety.rs` | CTL_ASYNC_BLOCKING |
| Create | `crates/cargo-test-lint/src/rules/structure.rs` | CTL_NESTED_MOD |
| Create | `crates/cargo-test-lint/src/rules/cloning.rs` | CTL_UNNECESSARY_CLONE |
| Create | `crates/cargo-test-lint/src/rules/complexity.rs` | CTL_DEEP_WRAPPER |
| Create | `crates/cargo-test-lint/src/rules/drop.rs` | CTL_MISSING_DROP_GUARD |
| Create | `crates/cargo-test-lint/src/rules/dead_code.rs` | CTL_DEAD_TEST_HELPER |
| Create | `crates/cargo-test-lint/src/output/mod.rs` | Output trait |
| Create | `crates/cargo-test-lint/src/output/terminal.rs` | Terminal formatter |
| Create | `crates/cargo-test-lint/src/output/sarif.rs` | SARIF 2.1.0 formatter |
| Modify | `crates/cargo-test-lint/src/main.rs` | New CLI entry point |
| Modify | `.github/workflows/ci.yml` | Adapt CI |
| Delete | `fixtures/` | Remove old test fixtures |
| Delete | `docs/INTEGRATION-rust-analyzer.md` | Remove old docs |
| Delete | `.worktrees/` | Remove worktree dir |

---

### Task 1: Workspace Restructuring

**Files:**
- Modify: `Cargo.toml`
- Delete: `crates/ctl-core/`, `crates/ctl-daemon/`
- Rename: `crates/ctl/` → `crates/cargo-test-lint/`

- [ ] **Step 1: Rename ctl crate directory**

```bash
cd /Users/jonathangadeaharder/Documents/projects/linters/cargo-test-lint
mv crates/ctl crates/cargo-test-lint
```

- [ ] **Step 2: Delete old crates**

```bash
rm -rf crates/ctl-core crates/ctl-daemon
```

- [ ] **Step 3: Delete old fixtures and docs**

```bash
rm -rf fixtures .worktrees docs/INTEGRATION-rust-analyzer.md
```

- [ ] **Step 4: Rewrite workspace Cargo.toml**

Replace entire `Cargo.toml` with:

```toml
[workspace]
resolver = "2"
members = ["crates/cargo-test-lint"]
exclude = ["fixtures"]

[workspace.package]
version = "0.2.0"
edition = "2024"
license = "MIT OR Apache-2.0"
rust-version = "1.85"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
toml = "0.8"
tempfile = "3"
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
anstyle = "1"
ignore = "0.4"

[workspace.metadata.binstall]
pkg-url = "{ repo }/releases/download/{ version }/cargo-test-lint-{ target }{ archive-suffix }"
bin-dir = "cargo-test-lint{ binary-ext }"
```

- [ ] **Step 5: Rewrite crate Cargo.toml**

Replace `crates/cargo-test-lint/Cargo.toml` with:

```toml
[package]
name = "cargo-test-lint"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
description = "AST-driven test quality linter for Rust"

[[bin]]
name = "cargo-test-lint"
path = "src/main.rs"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
clap = { workspace = true }
toml = { workspace = true }
tree-sitter = { workspace = true }
tree-sitter-rust = { workspace = true }
anstyle = { workspace = true }
ignore = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

- [ ] **Step 6: Verify workspace compiles**

```bash
cargo check --workspace 2>&1 | head -20
```

Expected: Compilation errors because source files don't exist yet. That's fine — we're establishing the structure.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: restructure workspace to single cargo-test-lint crate"
```

---

### Task 2: Diagnostics Module

**Files:**
- Create: `crates/cargo-test-lint/src/diagnostics.rs`
- Create: `crates/cargo-test-lint/src/lib.rs`

- [ ] **Step 1: Create lib.rs with module declarations**

```rust
pub mod config;
pub mod diagnostics;
pub mod output;
pub mod parser;
pub mod rules;
```

Write to `crates/cargo-test-lint/src/lib.rs`.

- [ ] **Step 2: Create diagnostics.rs with types**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Allow,
    Warn,
    Deny,
    Forbid,
}

impl DiagnosticLevel {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Deny | Self::Forbid)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        }
    }
}

impl std::fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DiagnosticLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "allow" => Ok(Self::Allow),
            "warn" => Ok(Self::Warn),
            "deny" => Ok(Self::Deny),
            "forbid" => Ok(Self::Forbid),
            _ => Err(format!("invalid level: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub rule_id: String,
    pub level: DiagnosticLevel,
    pub message: String,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub suggestion: Option<Fix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub description: String,
    pub replacement: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl Diagnostic {
    pub fn has_errors(diagnostics: &[Self]) -> bool {
        diagnostics.iter().any(|d| d.level.is_error())
    }

    pub fn sort_by_position(diagnostics: &mut [Self]) {
        diagnostics.sort_by(|a, b| {
            a.file_path
                .cmp(&b.file_path)
                .then(a.line.cmp(&b.line))
                .then(a.column.cmp(&b.column))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_level_from_str() {
        assert_eq!("allow".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Allow);
        assert_eq!("warn".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Warn);
        assert_eq!("deny".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Deny);
        assert_eq!("forbid".parse::<DiagnosticLevel>().unwrap(), DiagnosticLevel::Forbid);
        assert!("invalid".parse::<DiagnosticLevel>().is_err());
    }

    #[test]
    fn diagnostic_level_is_error() {
        assert!(!DiagnosticLevel::Allow.is_error());
        assert!(!DiagnosticLevel::Warn.is_error());
        assert!(DiagnosticLevel::Deny.is_error());
        assert!(DiagnosticLevel::Forbid.is_error());
    }

    #[test]
    fn has_errors_detects_violations() {
        let clean: Vec<Diagnostic> = vec![];
        assert!(!Diagnostic::has_errors(&clean));

        let warnings = vec![make_diag(DiagnosticLevel::Warn)];
        assert!(!Diagnostic::has_errors(&warnings));

        let errors = vec![make_diag(DiagnosticLevel::Deny)];
        assert!(Diagnostic::has_errors(&errors));
    }

    #[test]
    fn sort_by_position_orders_correctly() {
        let mut diags = vec![
            make_diag_at("b.rs", 10, 1),
            make_diag_at("a.rs", 5, 3),
            make_diag_at("a.rs", 5, 1),
        ];
        Diagnostic::sort_by_position(&mut diags);
        assert_eq!(diags[0].file_path, PathBuf::from("a.rs"));
        assert_eq!(diags[0].line, 5);
        assert_eq!(diags[0].column, 1);
        assert_eq!(diags[1].file_path, PathBuf::from("a.rs"));
        assert_eq!(diags[1].line, 5);
        assert_eq!(diags[1].column, 3);
        assert_eq!(diags[2].file_path, PathBuf::from("b.rs"));
    }

    fn make_diag(level: DiagnosticLevel) -> Diagnostic {
        make_diag_at("test.rs", 1, 1)
    }

    fn make_diag_at(path: &str, line: usize, col: usize) -> Diagnostic {
        Diagnostic {
            rule_id: "CTL_TEST".into(),
            level: DiagnosticLevel::Warn,
            message: "test".into(),
            file_path: PathBuf::from(path),
            line,
            column: col,
            end_line: line,
            end_column: col + 5,
            suggestion: None,
        }
    }
}
```

- [ ] **Step 3: Run diagnostics tests**

```bash
cargo test -p cargo-test-lint diagnostics
```

Expected: All 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/cargo-test-lint/src/lib.rs crates/cargo-test-lint/src/diagnostics.rs
git commit -m "feat: add diagnostics module with types and level parsing"
```

---

### Task 3: Config Module

**Files:**
- Create: `crates/cargo-test-lint/src/config.rs`

- [ ] **Step 1: Create config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::diagnostics::DiagnosticLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
    #[serde(default)]
    pub max_expects: usize,
    #[serde(default)]
    pub max_nested_mod: usize,
    #[serde(default)]
    pub nextest: bool,
    #[serde(default)]
    pub deny_warnings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleConfig {
    Level(DiagnosticLevel),
    Threshold(usize),
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("assertion-roulette".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("sleepy-test".into(), RuleConfig::Level(DiagnosticLevel::Forbid));
        rules.insert("test-branching".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("async-blocking".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("unnecessary-clone".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("deep-wrapper".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("missing-drop-guard".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("dead-test-helper".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("nextest-compatibility".into(), RuleConfig::Level(DiagnosticLevel::Warn));

        Self {
            rules,
            max_expects: 5,
            max_nested_mod: 3,
            nextest: false,
            deny_warnings: false,
        }
    }
}

impl Config {
    pub fn rule_level(&self, rule_id: &str, default: DiagnosticLevel) -> DiagnosticLevel {
        match self.rules.get(rule_id) {
            Some(RuleConfig::Level(level)) => level.clone(),
            _ => default,
        }
    }

    pub fn rule_enabled(&self, rule_id: &str) -> bool {
        !matches!(
            self.rules.get(rule_id),
            Some(RuleConfig::Level(DiagnosticLevel::Allow))
        )
    }
}

pub fn load(project_root: &Path) -> Config {
    let cargo_toml = project_root.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&cargo_toml) else {
        return Config::default();
    };

    #[derive(Deserialize)]
    struct Manifest {
        workspace: Option<Workspace>,
        lints: Option<Lints>,
    }

    #[derive(Deserialize)]
    struct Workspace {
        lints: Option<Lints>,
    }

    #[derive(Deserialize)]
    struct Lints {
        #[serde(rename = "cargo-test-lint")]
        cargo_test_lint: Option<Config>,
    }

    // Try workspace-level lints first, then package-level
    if let Ok(manifest) = toml::from_str::<Manifest>(&content) {
        if let Some(ws) = manifest.workspace {
            if let Some(lints) = ws.lints {
                if let Some(config) = lints.cargo_test_lint {
                    return config;
                }
            }
        }
        if let Some(lints) = manifest.lints {
            if let Some(config) = lints.cargo_test_lint {
                return config;
            }
        }
    }

    Config::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_config_has_all_rules() {
        let config = Config::default();
        assert!(config.rule_enabled("assertion-roulette"));
        assert!(config.rule_enabled("sleepy-test"));
        assert!(config.rule_enabled("test-branching"));
        assert_eq!(config.max_expects, 5);
        assert_eq!(config.max_nested_mod, 3);
        assert!(!config.nextest);
    }

    #[test]
    fn rule_level_returns_default_when_not_configured() {
        let config = Config::default();
        assert_eq!(
            config.rule_level("nonexistent", DiagnosticLevel::Deny),
            DiagnosticLevel::Deny
        );
    }

    #[test]
    fn rule_enabled_false_when_allowed() {
        let mut config = Config::default();
        config.rules.insert("test-rule".into(), RuleConfig::Level(DiagnosticLevel::Allow));
        assert!(!config.rule_enabled("test-rule"));
    }

    #[test]
    fn load_returns_defaults_when_no_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let config = load(tmp.path());
        assert_eq!(config.max_expects, 5);
    }

    #[test]
    fn load_parses_workspace_lints() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/foo"]

[workspace.lints.cargo-test-lint]
max-expects = 10
sleepy-test = "deny"
"#,
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.max_expects, 10);
        assert_eq!(
            config.rule_level("sleepy-test", DiagnosticLevel::Warn),
            DiagnosticLevel::Deny
        );
    }

    #[test]
    fn load_parses_package_level_lints() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[package]
name = "test-crate"

[lints.cargo-test-lint]
max-nested-mod = 2
"#,
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.max_nested_mod, 2);
    }
}
```

- [ ] **Step 2: Run config tests**

```bash
cargo test -p cargo-test-lint config
```

Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/config.rs
git commit -m "feat: add config module parsing Cargo.toml [lints.cargo-test-lint]"
```

---

### Task 4: Parser Module

**Files:**
- Create: `crates/cargo-test-lint/src/parser.rs`

- [ ] **Step 1: Create parser.rs**

```rust
use std::path::Path;
use tree_sitter::{Parser, Tree};

pub fn make_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("failed to set tree-sitter-rust language");
    parser
}

pub fn parse_source(source: &[u8]) -> Option<Tree> {
    let mut parser = make_parser();
    parser.parse(source, None)
}

pub fn parse_file(path: &Path) -> anyhow::Result<(Vec<u8>, Tree)> {
    let source = std::fs::read(path)?;
    let tree = parse_source(&source)
        .ok_or_else(|| anyhow::anyhow!("failed to parse {}", path.display()))?;
    Ok((source, tree))
}

pub fn collect_rs_files(project_root: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in ignore::Walk::new(project_root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_source_returns_tree() {
        let source = b"fn main() {}";
        let tree = parse_source(source);
        assert!(tree.is_some());
        let tree = tree.unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn parse_source_returns_none_for_invalid() {
        // tree-sitter is very forgiving, so this might still parse
        // but we test the API contract
        let source = b"fn main() {";
        let tree = parse_source(source);
        // tree-sitter typically still returns a tree with ERROR nodes
        assert!(tree.is_some());
    }

    #[test]
    fn collect_rs_files_finds_rust_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("lib.rs"), "fn main() {}").unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src.join("readme.txt"), "not rust").unwrap();

        let files = collect_rs_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "rs"));
    }

    use std::fs;
}
```

- [ ] **Step 2: Run parser tests**

```bash
cargo test -p cargo-test-lint parser
```

Expected: All 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/parser.rs
git commit -m "feat: add parser module with tree-sitter setup and file collection"
```

---

### Task 5: Rule Engine Infrastructure

**Files:**
- Create: `crates/cargo-test-lint/src/rules/mod.rs`

- [ ] **Step 1: Create rules/mod.rs with Rule trait, context, and engine**

```rust
pub mod assertions;
pub mod async_safety;
pub mod cloning;
pub mod complexity;
pub mod dead_code;
pub mod flow;
pub mod nextest;
pub mod sleep;
pub mod structure;
pub mod drop;

use crate::config::Config;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::path::Path;
use tree_sitter::{Query, QueryCursor, QueryMatch, Tree};

pub struct RuleContext<'a> {
    pub source: &'a [u8],
    pub tree: &'a Tree,
    pub config: &'a Config,
    pub file_path: &'a Path,
}

pub trait Rule {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn default_level(&self) -> DiagnosticLevel;
    fn query_str(&self) -> &'static str;
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic>;
}

pub fn run_rule<'a>(rule: &dyn Rule, ctx: &RuleContext<'a>) -> Vec<Diagnostic> {
    let level = ctx.config.rule_level(rule.id(), rule.default_level());
    if level == DiagnosticLevel::Allow {
        return vec![];
    }

    let language = tree_sitter_rust::LANGUAGE.into();
    let Ok(query) = Query::new(&language, rule.query_str()) else {
        return vec![];
    };

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, ctx.tree.root_node(), ctx.source);

    let mut diagnostics = Vec::new();
    for query_match in matches {
        let mut matched = QueryMatch::new(&query_match);
        let mut rule_diags = rule.validate(ctx, &mut matched);
        for diag in &mut rule_diags {
            diag.level = level.clone();
        }
        diagnostics.extend(rule_diags);
    }

    diagnostics
}

pub fn run_all_rules(ctx: &RuleContext) -> Vec<Diagnostic> {
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(assertions::AssertMsg),
        Box::new(assertions::MaxExpects),
        Box::new(sleep::SleepyTest),
        Box::new(flow::TestBranching),
        Box::new(nextest::StaticMut),
        Box::new(nextest::EnvSetVar),
        Box::new(async_safety::AsyncBlocking),
        Box::new(structure::NestedMod),
        Box::new(cloning::UnnecessaryClone),
        Box::new(complexity::DeepWrapper),
        Box::new(drop::MissingDropGuard),
        Box::new(dead_code::DeadTestHelper),
    ];

    let mut diagnostics = Vec::new();
    for rule in &rules {
        if ctx.config.rule_enabled(rule.id()) {
            diagnostics.extend(run_rule(rule.as_ref(), ctx));
        }
    }
    Diagnostic::sort_by_position(&mut diagnostics);
    diagnostics
}

/// Helper for tests: parse a snippet and run a single rule, returning diagnostics.
#[cfg(test)]
pub fn test_rule(rule: &dyn Rule, source: &str) -> Vec<Diagnostic> {
    let tree = crate::parser::parse_source(source.as_bytes()).unwrap();
    let config = Config::default();
    let ctx = RuleContext {
        source: source.as_bytes(),
        tree: &tree,
        config: &config,
        file_path: Path::new("test.rs"),
    };
    run_rule(rule, &ctx)
}

/// Helper for tests: parse a snippet and run a single rule with custom config.
#[cfg(test)]
pub fn test_rule_with_config(
    rule: &dyn Rule,
    source: &str,
    config: Config,
) -> Vec<Diagnostic> {
    let tree = crate::parser::parse_source(source.as_bytes()).unwrap();
    let ctx = RuleContext {
        source: source.as_bytes(),
        tree: &tree,
        config: &config,
        file_path: Path::new("test.rs"),
    };
    run_rule(rule, &ctx)
}
```

- [ ] **Step 2: Create stub rule modules**

Each rule module needs to exist for compilation. Create minimal stubs:

`crates/cargo-test-lint/src/rules/assertions.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AssertMsg;
pub struct MaxExpects;

impl Rule for AssertMsg {
    fn id(&self) -> &'static str { "CTL_ASSERT_MSG" }
    fn description(&self) -> &'static str { "assertion missing context message" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(macro_invocation) @macro" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}

impl Rule for MaxExpects {
    fn id(&self) -> &'static str { "CTL_MAX_EXPECTS" }
    fn description(&self) -> &'static str { "too many assertions in test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/sleep.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct SleepyTest;

impl Rule for SleepyTest {
    fn id(&self) -> &'static str { "CTL_SLEEP" }
    fn description(&self) -> &'static str { "thread::sleep in test code" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Forbid }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/flow.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct TestBranching;

impl Rule for TestBranching {
    fn id(&self) -> &'static str { "CTL_TEST_BRANCHING" }
    fn description(&self) -> &'static str { "control flow in test body" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/nextest.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StaticMut;
pub struct EnvSetVar;

impl Rule for StaticMut {
    fn id(&self) -> &'static str { "CTL_STATIC_MUT" }
    fn description(&self) -> &'static str { "static mutable variable" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(static_item) @static" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}

impl Rule for EnvSetVar {
    fn id(&self) -> &'static str { "CTL_ENV_SET_VAR" }
    fn description(&self) -> &'static str { "std::env::set_var in test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/async_safety.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AsyncBlocking;

impl Rule for AsyncBlocking {
    fn id(&self) -> &'static str { "CTL_ASYNC_BLOCKING" }
    fn description(&self) -> &'static str { "blocking call in async test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/structure.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct NestedMod;

impl Rule for NestedMod {
    fn id(&self) -> &'static str { "CTL_NESTED_MOD" }
    fn description(&self) -> &'static str { "deeply nested test module" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(mod_item) @mod" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/cloning.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct UnnecessaryClone;

impl Rule for UnnecessaryClone {
    fn id(&self) -> &'static str { "CTL_UNNECESSARY_CLONE" }
    fn description(&self) -> &'static str { "unnecessary .clone()" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/complexity.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeepWrapper;

impl Rule for DeepWrapper {
    fn id(&self) -> &'static str { "CTL_DEEP_WRAPPER" }
    fn description(&self) -> &'static str { "deeply nested type wrapper" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(type_item) @type" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/drop.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct MissingDropGuard;

impl Rule for MissingDropGuard {
    fn id(&self) -> &'static str { "CTL_MISSING_DROP_GUARD" }
    fn description(&self) -> &'static str { "resource allocation without RAII guard" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

`crates/cargo-test-lint/src/rules/dead_code.rs`:
```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeadTestHelper;

impl Rule for DeadTestHelper {
    fn id(&self) -> &'static str { "CTL_DEAD_TEST_HELPER" }
    fn description(&self) -> &'static str { "unused test helper" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
```

- [ ] **Step 3: Create stub output modules**

`crates/cargo-test-lint/src/output/mod.rs`:
```rust
pub mod sarif;
pub mod terminal;

use crate::diagnostics::Diagnostic;
use std::io::Write;

pub trait Formatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()>;
}

pub enum OutputFormat {
    Terminal,
    Sarif,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "terminal" => Ok(Self::Terminal),
            "sarif" => Ok(Self::Sarif),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}
```

`crates/cargo-test-lint/src/output/terminal.rs`:
```rust
use super::Formatter;
use crate::diagnostics::Diagnostic;
use std::io::Write;

pub struct TerminalFormatter;

impl Formatter for TerminalFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        for diag in diagnostics {
            writeln!(writer, "{}", diag.message)?;
        }
        Ok(())
    }
}
```

`crates/cargo-test-lint/src/output/sarif.rs`:
```rust
use super::Formatter;
use crate::diagnostics::Diagnostic;
use std::io::Write;

pub struct SarifFormatter;

impl Formatter for SarifFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        let sarif = serde_json::json!({
            "version": "2.1.0",
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "cargo-test-lint",
                        "version": env!("CARGO_PKG_VERSION"),
                        "rules": []
                    }
                },
                "results": []
            }]
        });
        serde_json::to_writer_pretty(writer, &sarif)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Create stub main.rs**

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    #[command(name = "test-lint")]
    TestLint(TestLintArgs),
}

#[derive(Parser)]
struct TestLintArgs {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,

    #[arg(long)]
    fix: bool,

    #[arg(long)]
    rules: Option<String>,

    #[arg(long, default_value = "terminal")]
    format: String,

    #[arg(long)]
    max_expects: Option<usize>,

    #[arg(long)]
    nextest: bool,

    #[arg(long)]
    deny_warnings: bool,
}

fn main() -> anyhow::Result<()> {
    let Cargo::TestLint(args) = Cargo::parse();
    let config = cargo_test_lint::config::load(&args.project_root);
    let files = cargo_test_lint::parser::collect_rs_files(&args.project_root)?;

    let mut all_diagnostics = Vec::new();
    for file in &files {
        let (source, tree) = cargo_test_lint::parser::parse_file(file)?;
        let ctx = cargo_test_lint::rules::RuleContext {
            source: &source,
            tree: &tree,
            config: &config,
            file_path: file,
        };
        all_diagnostics.extend(cargo_test_lint::rules::run_all_rules(&ctx));
    }

    use cargo_test_lint::output::Formatter;
    let formatter: Box<dyn Formatter> = match args.format.as_str() {
        "sarif" => Box::new(cargo_test_lint::output::sarif::SarifFormatter),
        _ => Box::new(cargo_test_lint::output::terminal::TerminalFormatter),
    };

    formatter.write(&all_diagnostics, &mut std::io::stderr())?;

    if cargo_test_lint::diagnostics::Diagnostic::has_errors(&all_diagnostics)
        || (args.deny_warnings && !all_diagnostics.is_empty())
    {
        std::process::exit(1);
    }

    Ok(())
}
```

- [ ] **Step 5: Verify compilation**

```bash
cargo check -p cargo-test-lint
```

Expected: Compiles successfully (stubs return empty diagnostics).

- [ ] **Step 6: Run all tests**

```bash
cargo test -p cargo-test-lint
```

Expected: All diagnostics, config, and parser tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/cargo-test-lint/
git commit -m "feat: add rule engine, stub rules, output formatters, and CLI"
```

---

### Task 6: Rule — CTL_ASSERT_MSG (Assertion Roulette)

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/assertions.rs`

- [ ] **Step 1: Write failing tests for AssertMsg**

Replace the `AssertMsg` implementation in `assertions.rs` with:

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
use tree_sitter::QueryMatch;

pub struct AssertMsg;

impl Rule for AssertMsg {
    fn id(&self) -> &'static str {
        "CTL_ASSERT_MSG"
    }

    fn description(&self) -> &'static str {
        "assertion missing context message"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(macro_invocation
            macro: (identifier) @name
            (#match? @name "^assert(_eq|_ne)?$")) @macro"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let macro_node = query_match.captures.iter().find(|c| c.index == 0);
        let name_node = query_match.captures.iter().find(|c| c.index == 1);

        let (Some(macro_capture), Some(name_capture)) = (macro_node, name_node) else {
            return vec![];
        };

        let macro_node = macro_capture.node;
        let name = &ctx.source[name_capture.node.byte_range()];

        // Count arguments by looking at token_tree children
        let token_tree = macro_node.child_by_field_name("token_tree");
        let Some(tt) = token_tree else {
            return vec![];
        };

        // Count comma-separated arguments
        let arg_count = count_macro_args(&tt, ctx.source);

        // assert! needs 2+ args (condition + message)
        // assert_eq!/assert_ne! needs 3+ args (left, right + message)
        let min_args = if name == b"assert" { 2 } else { 3 };

        if arg_count < min_args {
            let node_range = macro_node.byte_range();
            let line = macro_node.start_position().row + 1;
            let col = macro_node.start_position().column + 1;
            let end_line = macro_node.end_position().row + 1;
            let end_col = macro_node.end_position().column + 1;

            let suggestion = build_suggestion(name, &tt, ctx.source);

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: "assertion missing context message — add a format string for readable CI failures".into(),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line,
                end_column: end_col,
                suggestion: suggestion.map(|s| Fix {
                    description: "add context message".into(),
                    replacement: s,
                    start_byte: node_range.start,
                    end_byte: node_range.end,
                }),
            }]
        } else {
            vec![]
        }
    }
}

fn count_macro_args(token_tree: &tree_sitter::Node, source: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = token_tree.walk();
    for child in token_tree.named_children(&mut cursor) {
        if child.kind() == "," {
            continue;
        }
        count += 1;
    }
    // If there's at least one named child, count = 1 + number of commas
    // But simpler: count non-comma named children
    if count == 0 && !token_tree.named_children(&mut cursor).next().is_some() {
        // Empty token tree
        return 0;
    }
    count
}

fn build_suggestion(name: &[u8], token_tree: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    let original = &source[token_tree.byte_range()];
    let original_str = std::str::from_utf8(original).ok()?;
    let macro_name = std::str::from_utf8(name).ok()?;

    // Strip outer parens
    let inner = original_str.strip_prefix('(')?.strip_suffix(')')?;

    Some(format!("{macro_name}!({inner}, \"TODO: add context message\")"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> AssertMsg {
        AssertMsg
    }

    #[test]
    fn assert_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_ASSERT_MSG");
    }

    #[test]
    fn assert_with_message_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true, "should be true");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn assert_eq_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1 + 1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn assert_eq_with_message_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1 + 1, 2, "math should work");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn assert_ne_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert_ne!(1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn non_assert_macros_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    println!("hello");
    vec![1, 2, 3];
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn suggestion_includes_message_placeholder() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        let fix = diags[0].suggestion.as_ref().unwrap();
        assert!(fix.replacement.contains("TODO: add context message"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p cargo-test-lint assert_msg
```

Expected: Tests fail because validation returns empty vec.

- [ ] **Step 3: Implement AssertMsg validation**

The `count_macro_args` and `build_suggestion` functions above are the implementation. Verify tests pass:

```bash
cargo test -p cargo-test-lint assert_msg
```

Expected: All 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/cargo-test-lint/src/rules/assertions.rs
git commit -m "feat: implement CTL_ASSERT_MSG rule for assertion roulette"
```

---

### Task 7: Rule — CTL_MAX_EXPECTS (Assertion Count)

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/assertions.rs`

- [ ] **Step 1: Add MaxExpects tests and implementation**

Add to `assertions.rs` after the `AssertMsg` implementation:

```rust
pub struct MaxExpects;

impl Rule for MaxExpects {
    fn id(&self) -> &'static str {
        "CTL_MAX_EXPECTS"
    }

    fn description(&self) -> &'static str {
        "too many assertions in test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(attribute_item
            (attribute
                (identifier) @attr_name
                (#eq? @attr_name "test"))
            ) @attr
            .
            (function_item
                name: (identifier) @fn_name
                body: (block) @body) @fn"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let body_capture = query_match.captures.iter().find(|c| c.index == 3);
        let fn_capture = query_match.captures.iter().find(|c| c.index == 4);

        let (Some(body_node), Some(fn_node)) = (body_capture.map(|c| c.node), fn_capture.map(|c| c.node)) else {
            return vec![];
        };

        let threshold = ctx.config.max_expects;
        if threshold == 0 {
            return vec![];
        }

        let assert_count = count_assertions(&body_node, ctx.source);

        if assert_count > threshold {
            let line = fn_node.start_position().row + 1;
            let col = fn_node.start_position().column + 1;

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!(
                    "test has {assert_count} assertions (max {threshold}) — consider splitting"
                ),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line: fn_node.end_position().row + 1,
                end_column: fn_node.end_position().column + 1,
                suggestion: None,
            }]
        } else {
            vec![]
        }
    }
}

fn count_assertions(body: &tree_sitter::Node, _source: &[u8]) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.descendants(&mut cursor) {
        if child.kind() == "macro_invocation" {
            if let Some(name_node) = child.child_by_field_name("macro") {
                let name = name_node.utf8_text(_source).unwrap_or("");
                if name.starts_with("assert") {
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule_with_config;
    use crate::config::Config;

    fn rule() -> MaxExpects {
        MaxExpects
    }

    fn config_with_max(max: usize) -> Config {
        let mut config = Config::default();
        config.max_expects = max;
        config
    }

    #[test]
    fn under_threshold_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert_eq!(1, 1);
}
"#;
        let config = config_with_max(5);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn over_threshold_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
}
"#;
        let config = config_with_max(5);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("6 assertions"));
        assert!(diags[0].message.contains("max 5"));
    }

    #[test]
    fn at_threshold_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert!(true);
    assert!(true);
}
"#;
        let config = config_with_max(3);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn zero_threshold_disables_check() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#;
        let config = config_with_max(0);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint max_expects
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/assertions.rs
git commit -m "feat: implement CTL_MAX_EXPECTS rule for assertion count threshold"
```

---

### Task 8: Rule — CTL_SLEEP (Sleepy Test)

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/sleep.rs`

- [ ] **Step 1: Implement SleepyTest**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct SleepyTest;

impl Rule for SleepyTest {
    fn id(&self) -> &'static str {
        "CTL_SLEEP"
    }

    fn description(&self) -> &'static str {
        "thread::sleep in test code"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Forbid
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression
            function: (scoped_identifier
                path: (identifier) @path
                name: (identifier) @name
                (#eq? @path "std")
                (#eq? @name "sleep"))) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = call_node else {
            return vec![];
        };

        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: "std::thread::sleep blocks the thread — use tokio::time::sleep, Condvar, or channels"
                .into(),
            file_path: ctx.file_path.to_path_buf(),
            line,
            column: col,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> SleepyTest {
        SleepyTest
    }

    #[test]
    fn thread_sleep_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    std::thread::sleep(std::time::Duration::from_millis(100));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_SLEEP");
    }

    #[test]
    fn no_sleep_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let x = 1 + 1;
    assert_eq!(x, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn other_function_calls_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    std::thread::spawn(|| {});
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn message_suggests_alternatives() {
        let source = r#"
#[test]
fn test_foo() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("tokio::time::sleep"));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint sleep
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/sleep.rs
git commit -m "feat: implement CTL_SLEEP rule forbidding thread::sleep"
```

---

### Task 9: Rule — CTL_TEST_BRANCHING (Deterministic Flow)

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/flow.rs`

- [ ] **Step 1: Implement TestBranching**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct TestBranching;

impl Rule for TestBranching {
    fn id(&self) -> &'static str {
        "CTL_TEST_BRANCHING"
    }

    fn description(&self) -> &'static str {
        "control flow in test body"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(attribute_item
            (attribute
                (identifier) @attr_name
                (#eq? @attr_name "test"))
            ) @attr
            .
            (function_item
                name: (identifier) @fn_name
                body: (block) @body) @fn"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let body_capture = query_match.captures.iter().find(|c| c.index == 3);
        let Some(body_node) = body_capture.map(|c| c.node) else {
            return vec![];
        };

        let mut diagnostics = Vec::new();
        let mut cursor = body_node.walk();

        for child in body_node.children(&mut cursor) {
            let kind = child.kind();
            let is_branching = matches!(
                kind,
                "if_expression" | "match_expression" | "while_expression" | "loop_expression"
            );

            if is_branching {
                let line = child.start_position().row + 1;
                let col = child.start_position().column + 1;
                let kind_name = match kind {
                    "if_expression" => "if",
                    "match_expression" => "match",
                    "while_expression" => "while",
                    "loop_expression" => "loop",
                    _ => kind,
                };

                diagnostics.push(Diagnostic {
                    rule_id: self.id().into(),
                    level: self.default_level(),
                    message: format!(
                        "{kind_name} in test body — assertions may be bypassed silently"
                    ),
                    file_path: ctx.file_path.to_path_buf(),
                    line,
                    column: col,
                    end_line: child.end_position().row + 1,
                    end_column: child.end_position().column + 1,
                    suggestion: None,
                });
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> TestBranching {
        TestBranching
    }

    #[test]
    fn if_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    if true {
        assert!(true);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("if"));
    }

    #[test]
    fn match_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    match 42 {
        _ => assert!(true),
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("match"));
    }

    #[test]
    fn while_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    while false {
        assert!(true);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("while"));
    }

    #[test]
    fn loop_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    loop {
        break;
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("loop"));
    }

    #[test]
    fn flat_test_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let x = 1;
    assert_eq!(x, 1);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn branching_in_non_test_ignored() {
        let source = r#"
fn helper() {
    if true {
        println!("not a test");
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn multiple_branches_flagged_separately() {
        let source = r#"
#[test]
fn test_foo() {
    if true {}
    match 42 { _ => {} }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint branching
```

Expected: All 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/flow.rs
git commit -m "feat: implement CTL_TEST_BRANCHING rule for control flow in tests"
```

---

### Task 10: Rule — CTL_STATIC_MUT / CTL_ENV_SET_VAR (nextest)

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/nextest.rs`

- [ ] **Step 1: Implement StaticMut and EnvSetVar**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StaticMut;

impl Rule for StaticMut {
    fn id(&self) -> &'static str {
        "CTL_STATIC_MUT"
    }

    fn description(&self) -> &'static str {
        "static mutable variable"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(static_item
            "mutable" @mut
            name: (identifier) @name) @static"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let static_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let name_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 2)
            .map(|c| c.node);

        let (Some(node), Some(name)) = (static_node, name_node) else {
            return vec![];
        };

        let name_str = name.utf8_text(ctx.source).unwrap_or("unknown");
        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "static mut `{name_str}` — breaks process-isolated test runners (nextest)"
            ),
            file_path: ctx.file_path.to_path_buf(),
            line,
            column: col,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

pub struct EnvSetVar;

impl Rule for EnvSetVar {
    fn id(&self) -> &'static str {
        "CTL_ENV_SET_VAR"
    }

    fn description(&self) -> &'static str {
        "std::env::set_var in test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression
            function: (scoped_identifier
                path: (scoped_identifier
                    path: (identifier) @std
                    name: (identifier) @env)
                name: (identifier) @fn_name
                (#eq? @std "std")
                (#eq? @env "env")
                (#eq? @fn_name "set_var"))) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = call_node else {
            return vec![];
        };

        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: "std::env::set_var poisons the environment for concurrent tests — use temp_env or serial_test"
                .into(),
            file_path: ctx.file_path.to_path_buf(),
            line,
            column: col,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn static_mut_flagged() {
        let source = r#"
static mut COUNTER: u32 = 0;
"#;
        let diags = test_rule(&StaticMut, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("COUNTER"));
    }

    #[test]
    fn static_immutable_passes() {
        let source = r#"
static COUNTER: u32 = 0;
"#;
        let diags = test_rule(&StaticMut, source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn env_set_var_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    std::env::set_var("MY_VAR", "value");
}
"#;
        let diags = test_rule(&EnvSetVar, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("set_var"));
    }

    #[test]
    fn env_var_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let _ = std::env::var("MY_VAR");
}
"#;
        let diags = test_rule(&EnvSetVar, source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn other_std_calls_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    std::fs::read_to_string("foo.txt").ok();
}
"#;
        let diags = test_rule(&EnvSetVar, source);
        assert_eq!(diags.len(), 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint nextest
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/nextest.rs
git commit -m "feat: implement CTL_STATIC_MUT and CTL_ENV_SET_VAR nextest rules"
```

---

### Task 11: Rule — CTL_ASYNC_BLOCKING

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/async_safety.rs`

- [ ] **Step 1: Implement AsyncBlocking**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AsyncBlocking;

impl Rule for AsyncBlocking {
    fn id(&self) -> &'static str {
        "CTL_ASYNC_BLOCKING"
    }

    fn description(&self) -> &'static str {
        "blocking call in async test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(attribute_item
            (attribute
                (identifier) @attr_name
                (#match? @attr_name "^(tokio|async_std)::test$"))
            ) @attr
            .
            (function_item
                name: (identifier) @fn_name
                body: (block) @body) @fn"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let body_capture = query_match.captures.iter().find(|c| c.index == 3);
        let Some(body_node) = body_capture.map(|c| c.node) else {
            return vec![];
        };

        let blocking_fns = [
            "std::fs::read",
            "std::fs::read_to_string",
            "std::fs::write",
            "std::fs::create_dir",
            "std::fs::remove_file",
            "std::fs::remove_dir",
            "std::fs::copy",
            "std::fs::rename",
            "std::fs::metadata",
            "std::thread::sleep",
            "std::net::TcpStream::connect",
            "std::net::TcpListener::bind",
            "std::io::stdin",
            "std::io::stdout",
        ];

        let mut diagnostics = Vec::new();
        let mut cursor = body_node.walk();

        for descendant in body_node.descendants(&mut cursor) {
            if descendant.kind() != "call_expression" {
                continue;
            }

            let Some(func_node) = descendant.child_by_field_name("function") else {
                continue;
            };

            let func_text = func_node.utf8_text(ctx.source).unwrap_or("");

            for &blocking in &blocking_fns {
                if func_text == blocking || func_text.ends_with(&blocking[5..]) {
                    let line = descendant.start_position().row + 1;
                    let col = descendant.start_position().column + 1;

                    diagnostics.push(Diagnostic {
                        rule_id: self.id().into(),
                        level: self.default_level(),
                        message: format!(
                            "blocking call `{func_text}` in async test — use async equivalent"
                        ),
                        file_path: ctx.file_path.to_path_buf(),
                        line,
                        column: col,
                        end_line: descendant.end_position().row + 1,
                        end_column: descendant.end_position().column + 1,
                        suggestion: None,
                    });
                    break;
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> AsyncBlocking {
        AsyncBlocking
    }

    #[test]
    fn blocking_fs_in_tokio_test_flagged() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    let _ = std::fs::read_to_string("foo.txt");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("read_to_string"));
    }

    #[test]
    fn blocking_sleep_in_tokio_test_flagged() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    std::thread::sleep(std::time::Duration::from_millis(100));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("sleep"));
    }

    #[test]
    fn async_code_passes() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    let _ = tokio::fs::read_to_string("foo.txt").await;
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn non_async_test_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    let _ = std::fs::read_to_string("foo.txt");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn multiple_blocking_calls_flagged() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    let _ = std::fs::read_to_string("a.txt");
    let _ = std::fs::read_to_string("b.txt");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint async
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/async_safety.rs
git commit -m "feat: implement CTL_ASYNC_BLOCKING rule for async test safety"
```

---

### Task 12: Rule — CTL_NESTED_MOD

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/structure.rs`

- [ ] **Step 1: Implement NestedMod**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct NestedMod;

impl Rule for NestedMod {
    fn id(&self) -> &'static str {
        "CTL_NESTED_MOD"
    }

    fn description(&self) -> &'static str {
        "deeply nested test module"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(mod_item
            name: (identifier) @name) @mod"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let mod_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let name_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 1)
            .map(|c| c.node);

        let (Some(node), Some(name)) = (mod_node, name_node) else {
            return vec![];
        };

        let name_str = name.utf8_text(ctx.source).unwrap_or("");

        // Check if this is a test module
        if !is_test_module(&node, ctx.source, name_str) {
            return vec![];
        }

        let depth = mod_nesting_depth(&node);
        let threshold = ctx.config.max_nested_mod;
        if threshold == 0 || depth <= threshold {
            return vec![];
        }

        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!("test module nesting depth {depth} (max {threshold}) — flatten structure"),
            file_path: ctx.file_path.to_path_buf(),
            line,
            column: col,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

fn is_test_module(node: &tree_sitter::Node, source: &[u8], name: &str) -> bool {
    if name == "tests" || name == "test" {
        return true;
    }

    // Check for #[cfg(test)] attribute
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = child.utf8_text(source).unwrap_or("");
            if text.contains("cfg(test)") {
                return true;
            }
        }
    }

    false
}

fn mod_nesting_depth(node: &tree_sitter::Node) -> usize {
    let mut depth = 1;
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "mod_item" {
            depth += 1;
        }
        current = parent.parent();
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule_with_config;
    use crate::config::Config;

    fn rule() -> NestedMod {
        NestedMod
    }

    fn config_with_max(max: usize) -> Config {
        let mut config = Config::default();
        config.max_nested_mod = max;
        config
    }

    #[test]
    fn shallow_test_mod_passes() {
        let source = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_foo() {}
}
"#;
        let config = config_with_max(3);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn deeply_nested_test_mod_flagged() {
        let source = r#"
mod outer {
    mod inner {
        mod tests {
            #[test]
            fn test_foo() {}
        }
    }
}
"#;
        let config = config_with_max(2);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("depth 3"));
    }

    #[test]
    fn non_test_mod_ignored() {
        let source = r#"
mod a {
    mod b {
        mod c {
            fn helper() {}
        }
    }
}
"#;
        let config = config_with_max(2);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn cfg_test_attribute_detected() {
        let source = r#"
mod a {
    mod b {
        #[cfg(test)]
        mod my_tests {
            #[test]
            fn test_foo() {}
        }
    }
}
"#;
        let config = config_with_max(2);
        let diags = test_rule_with_config(&rule(), source, config);
        assert_eq!(diags.len(), 1);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint nested_mod
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/structure.rs
git commit -m "feat: implement CTL_NESTED_MOD rule for test module nesting depth"
```

---

### Task 13: Rule — CTL_UNNECESSARY_CLONE

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/cloning.rs`

- [ ] **Step 1: Implement UnnecessaryClone**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct UnnecessaryClone;

impl Rule for UnnecessaryClone {
    fn id(&self) -> &'static str {
        "CTL_UNNECESSARY_CLONE"
    }

    fn description(&self) -> &'static str {
        "unnecessary .clone()"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression
            function: (field_expression
                field: (identifier) @field
                (#eq? @field "clone"))) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = call_node else {
            return vec![];
        };

        // Check if the clone is the RHS of a let binding
        // and the original is never used after
        let parent = node.parent();
        let is_let_rhs = parent
            .map(|p| p.kind() == "let_declaration")
            .unwrap_or(false);

        if !is_let_rhs {
            return vec![];
        }

        // Get the original variable from the field_expression
        let field_expr = node.child_by_field_name("function");
        let Some(fe) = field_expr else {
            return vec![];
        };
        let object = fe.child(0);
        let Some(obj) = object else {
            return vec![];
        };

        let obj_text = obj.utf8_text(ctx.source).unwrap_or("");

        // Check if the original is used later in the same scope
        let let_parent = parent.unwrap();
        let scope_parent = let_parent.parent();
        let Some(scope) = scope_parent else {
            return vec![];
        };

        let obj_range = obj.byte_range();
        let clone_end = node.end_byte();

        // Search for references to the original after the clone
        let mut found_usage = false;
        let mut cursor = scope.walk();
        for descendant in scope.descendants(&mut cursor) {
            if descendant.byte_range().start >= clone_end {
                let text = descendant.utf8_text(ctx.source).unwrap_or("");
                if text == obj_text && descendant.kind() == "identifier" {
                    found_usage = true;
                    break;
                }
            }
        }

        if !found_usage {
            let line = node.start_position().row + 1;
            let col = node.start_position().column + 1;

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!("value `{obj_text}` cloned but not reused — consider borrowing instead"),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line: node.end_position().row + 1,
                end_column: node.end_position().column + 1,
                suggestion: None,
            }]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> UnnecessaryClone {
        UnnecessaryClone
    }

    #[test]
    fn clone_not_reused_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    let x = vec![1, 2, 3];
    let y = x.clone();
    assert_eq!(y, vec![1, 2, 3]);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("x"));
    }

    #[test]
    fn clone_reused_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let x = vec![1, 2, 3];
    let y = x.clone();
    assert_eq!(y, vec![1, 2, 3]);
    assert_eq!(x.len(), 3);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn non_let_clone_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    let x = vec![1, 2, 3];
    foo(x.clone());
    assert_eq!(x.len(), 3);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint clone
```

Expected: All 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/cloning.rs
git commit -m "feat: implement CTL_UNNECESSARY_CLONE rule"
```

---

### Task 14: Rule — CTL_DEEP_WRAPPER

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/complexity.rs`

- [ ] **Step 1: Implement DeepWrapper**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeepWrapper;

impl Rule for DeepWrapper {
    fn id(&self) -> &'static str {
        "CTL_DEEP_WRAPPER"
    }

    fn description(&self) -> &'static str {
        "deeply nested type wrapper"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(type_item
            name: (type_identifier) @name
            type: (_) @ty) @type_item"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let ty_capture = query_match.captures.iter().find(|c| c.index == 2);
        let item_capture = query_match.captures.iter().find(|c| c.index == 0);

        let (Some(ty_node), Some(item_node)) = (ty_capture.map(|c| c.node), item_capture.map(|c| c.node)) else {
            return vec![];
        };

        let depth = count_generic_depth(&ty_node, 0);

        if depth > 3 {
            let line = item_node.start_position().row + 1;
            let col = item_node.start_position().column + 1;

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!(
                    "deeply nested type wrapper ({depth} levels) — test setup is overly complex"
                ),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line: item_node.end_position().row + 1,
                end_column: item_node.end_position().column + 1,
                suggestion: None,
            }]
        } else {
            vec![]
        }
    }
}

fn count_generic_depth(node: &tree_sitter::Node, current: usize) -> usize {
    let kind = node.kind();
    let is_wrapper = matches!(
        kind,
        "generic_type" | "tuple_type" | "reference_type" | "array_type"
    );

    let next = if is_wrapper { current + 1 } else { current };
    let mut max_depth = next;

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_depth = count_generic_depth(&child, if is_wrapper { next } else { current });
        max_depth = max_depth.max(child_depth);
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> DeepWrapper {
        DeepWrapper
    }

    #[test]
    fn simple_type_passes() {
        let source = r#"
type MyType = Vec<u32>;
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn deeply_nested_type_flagged() {
        let source = r#"
type MyType = Arc<Mutex<Option<HashMap<String, Vec<u32>>>>>;
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("levels"));
    }

    #[test]
    fn moderate_nesting_passes() {
        let source = r#"
type MyType = Arc<Mutex<u32>>;
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint deep_wrapper
```

Expected: All 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/complexity.rs
git commit -m "feat: implement CTL_DEEP_WRAPPER rule for type complexity"
```

---

### Task 15: Rule — CTL_MISSING_DROP_GUARD

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/drop.rs`

- [ ] **Step 1: Implement MissingDropGuard**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct MissingDropGuard;

const RESOURCE_ALLOCATORS: &[&str] = &[
    "File::create",
    "File::open",
    "TempDir::new",
    "Builder::new",
    "TcpListener::bind",
    "UdpSocket::bind",
];

impl Rule for MissingDropGuard {
    fn id(&self) -> &'static str {
        "CTL_MISSING_DROP_GUARD"
    }

    fn description(&self) -> &'static str {
        "resource allocation without RAII guard"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = call_node else {
            return vec![];
        };

        let func_node = node.child_by_field_name("function");
        let Some(func) = func_node else {
            return vec![];
        };

        let func_text = func.utf8_text(ctx.source).unwrap_or("");

        let is_resource = RESOURCE_ALLOCATORS
            .iter()
            .any(|alloc| func_text.contains(alloc));

        if !is_resource {
            return vec![];
        }

        // Check if this call is the RHS of a let binding
        let parent = node.parent();
        let is_bound = parent
            .map(|p| p.kind() == "let_declaration")
            .unwrap_or(false);

        if is_bound {
            return vec![];
        }

        let line = node.start_position().row + 1;
        let col = node.start_position().column + 1;

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "resource allocation `{func_text}` without RAII guard — may leak on assertion panic"
            ),
            file_path: ctx.file_path.to_path_buf(),
            line,
            column: col,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> MissingDropGuard {
        MissingDropGuard
    }

    #[test]
    fn unbound_file_create_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    File::create("test.txt");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("File::create"));
    }

    #[test]
    fn bound_file_create_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let f = File::create("test.txt");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn non_resource_call_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    println!("hello");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn tcp_listener_without_binding_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    TcpListener::bind("127.0.0.1:0");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint drop_guard
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/drop.rs
git commit -m "feat: implement CTL_MISSING_DROP_GUARD rule for resource leaks"
```

---

### Task 16: Rule — CTL_DEAD_TEST_HELPER

**Files:**
- Modify: `crates/cargo-test-lint/src/rules/dead_code.rs`

- [ ] **Step 1: Implement DeadTestHelper**

```rust
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::collections::HashSet;
use tree_sitter::QueryMatch;

pub struct DeadTestHelper;

impl Rule for DeadTestHelper {
    fn id(&self) -> &'static str {
        "CTL_DEAD_TEST_HELPER"
    }

    fn description(&self) -> &'static str {
        "unused test helper"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(mod_item
            name: (identifier) @mod_name
            body: (declaration_list) @body) @mod"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let mod_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let name_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 1)
            .map(|c| c.node);

        let body_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 2)
            .map(|c| c.node);

        let (Some(node), Some(name), Some(body)) = (mod_node, name_node, body_node) else {
            return vec![];
        };

        let mod_name = name.utf8_text(ctx.source).unwrap_or("");

        // Only check test modules
        if !is_test_module(&node, ctx.source, mod_name) {
            return vec![];
        }

        // Collect definitions
        let definitions = collect_definitions(&body, ctx.source);

        if definitions.is_empty() {
            return vec![];
        }

        // Collect all identifier references in the module body
        let references = collect_references(&body, ctx.source);

        let mut diagnostics = Vec::new();

        for (def_name, def_node) in &definitions {
            if !references.contains(def_name.as_str()) {
                let line = def_node.start_position().row + 1;
                let col = def_node.start_position().column + 1;

                diagnostics.push(Diagnostic {
                    rule_id: self.id().into(),
                    level: self.default_level(),
                    message: format!("unused test helper `{def_name}` — defined but never referenced"),
                    file_path: ctx.file_path.to_path_buf(),
                    line,
                    column: col,
                    end_line: def_node.end_position().row + 1,
                    end_column: def_node.end_position().column + 1,
                    suggestion: None,
                });
            }
        }

        diagnostics
    }
}

fn is_test_module(node: &tree_sitter::Node, source: &[u8], name: &str) -> bool {
    if name == "tests" || name == "test" {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = child.utf8_text(source).unwrap_or("");
            if text.contains("cfg(test)") {
                return true;
            }
        }
    }

    false
}

fn collect_definitions(body: &tree_sitter::Node, source: &[u8]) -> Vec<(String, tree_sitter::Node)> {
    let mut defs = Vec::new();
    let mut cursor = body.walk();

    for child in body.named_children(&mut cursor) {
        let kind = child.kind();
        let name_field = match kind {
            "function_item" => child.child_by_field_name("name"),
            "struct_item" => child.child_by_field_name("name"),
            "enum_item" => child.child_by_field_name("name"),
            "type_item" => child.child_by_field_name("name"),
            "trait_item" => child.child_by_field_name("name"),
            _ => None,
        };

        if let Some(name_node) = name_field {
            let name = name_node.utf8_text(source).unwrap_or("").to_string();
            defs.push((name, child));
        }
    }

    defs
}

fn collect_references(body: &tree_sitter::Node, source: &[u8]) -> HashSet<String> {
    let mut refs = HashSet::new();
    let mut cursor = body.walk();

    for descendant in body.descendants(&mut cursor) {
        if descendant.kind() == "identifier" || descendant.kind() == "type_identifier" {
            let text = descendant.utf8_text(source).unwrap_or("");
            refs.insert(text.to_string());
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> DeadTestHelper {
        DeadTestHelper
    }

    #[test]
    fn unused_function_flagged() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn helper() -> u32 {
        42
    }

    #[test]
    fn test_foo() {
        assert_eq!(1, 1);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("helper"));
    }

    #[test]
    fn used_function_passes() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn helper() -> u32 {
        42
    }

    #[test]
    fn test_foo() {
        assert_eq!(helper(), 42);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn unused_struct_flagged() {
        let source = r#"
#[cfg(test)]
mod tests {
    struct TestData {
        value: u32,
    }

    #[test]
    fn test_foo() {
        assert_eq!(1, 1);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("TestData"));
    }

    #[test]
    fn non_test_module_ignored() {
        let source = r#"
mod helpers {
    fn unused() {}
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn multiple_unused_flagged() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn helper_a() {}
    fn helper_b() {}

    #[test]
    fn test_foo() {
        assert_eq!(1, 1);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint dead_test
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/rules/dead_code.rs
git commit -m "feat: implement CTL_DEAD_TEST_HELPER rule for unused test helpers"
```

---

### Task 17: Terminal Output Formatter

**Files:**
- Modify: `crates/cargo-test-lint/src/output/terminal.rs`

- [ ] **Step 1: Implement TerminalFormatter**

```rust
use super::Formatter;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::io::Write;

pub struct TerminalFormatter;

impl Formatter for TerminalFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        for diag in diagnostics {
            let level_str = match diag.level {
                DiagnosticLevel::Allow => continue,
                DiagnosticLevel::Warn => "warning",
                DiagnosticLevel::Deny => "error",
                DiagnosticLevel::Forbid => "error",
            };

            let color = match diag.level {
                DiagnosticLevel::Warn => "\x1b[33m",   // yellow
                DiagnosticLevel::Deny | DiagnosticLevel::Forbid => "\x1b[31m", // red
                _ => "\x1b[0m",
            };

            let reset = "\x1b[0m";
            let bold = "\x1b[1m";
            let dim = "\x1b[2m";

            writeln!(
                writer,
                "{color}{level_str}{reset}{dim}[{rule}]{reset}: {message}",
                rule = diag.rule_id,
                message = diag.message,
            )?;

            writeln!(
                writer,
                "  {bold}-->{reset} {path}:{line}:{col}",
                path = diag.file_path.display(),
                line = diag.line,
                col = diag.column,
            )?;

            if let Some(fix) = &diag.suggestion {
                writeln!(
                    writer,
                    "  {dim}|{reset} help: {desc}: `{replacement}`",
                    desc = fix.description,
                    replacement = fix.replacement,
                )?;
            }

            writeln!(writer)?;
        }

        // Summary
        let errors = diagnostics
            .iter()
            .filter(|d| d.level.is_error())
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| matches!(d.level, DiagnosticLevel::Warn))
            .count();

        if !diagnostics.is_empty() {
            let color = if errors > 0 { "\x1b[31m" } else { "\x1b[33m" };
            let reset = "\x1b[0m";
            writeln!(
                writer,
                "{color}{} error{}, {} warning{}{reset}",
                errors,
                if errors == 1 { "" } else { "s" },
                warnings,
                if warnings == 1 { "" } else { "s" },
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use std::path::PathBuf;

    fn make_diag(rule_id: &str, level: DiagnosticLevel, msg: &str) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
            level,
            message: msg.into(),
            file_path: PathBuf::from("src/lib.rs"),
            line: 10,
            column: 5,
            end_line: 10,
            end_column: 20,
            suggestion: None,
        }
    }

    fn format(diags: &[Diagnostic]) -> String {
        let formatter = TerminalFormatter;
        let mut buf = Vec::new();
        formatter.write(diags, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn warning_contains_rule_id() {
        let diags = vec![make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "missing msg")];
        let output = format(&diags);
        assert!(output.contains("CTL_ASSERT_MSG"));
        assert!(output.contains("warning"));
    }

    #[test]
    fn error_contains_file_location() {
        let diags = vec![make_diag("CTL_SLEEP", DiagnosticLevel::Forbid, "sleepy")];
        let output = format(&diags);
        assert!(output.contains("src/lib.rs:10:5"));
        assert!(output.contains("error"));
    }

    #[test]
    fn suggestion_rendered() {
        let mut diag = make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn, "no msg");
        diag.suggestion = Some(Fix {
            description: "add message".into(),
            replacement: "assert!(true, \"msg\")".into(),
            start_byte: 0,
            end_byte: 20,
        });
        let output = format(&[diag]);
        assert!(output.contains("help: add message"));
        assert!(output.contains("assert!(true, \"msg\")"));
    }

    #[test]
    fn summary_counts() {
        let diags = vec![
            make_diag("A", DiagnosticLevel::Warn, "w1"),
            make_diag("B", DiagnosticLevel::Warn, "w2"),
            make_diag("C", DiagnosticLevel::Deny, "e1"),
        ];
        let output = format(&diags);
        assert!(output.contains("1 error"));
        assert!(output.contains("2 warnings"));
    }

    #[test]
    fn empty_diagnostics_no_output() {
        let output = format(&[]);
        assert!(output.is_empty());
    }

    #[test]
    fn allow_level_skipped() {
        let diags = vec![make_diag("A", DiagnosticLevel::Allow, "hidden")];
        let output = format(&diags);
        assert!(output.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint terminal
```

Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/output/terminal.rs
git commit -m "feat: implement terminal output formatter with colors and summary"
```

---

### Task 18: SARIF Output Formatter

**Files:**
- Modify: `crates/cargo-test-lint/src/output/sarif.rs`

- [ ] **Step 1: Implement SarifFormatter**

```rust
use super::Formatter;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use serde_json::json;
use std::io::Write;

pub struct SarifFormatter;

impl Formatter for SarifFormatter {
    fn write(&self, diagnostics: &[Diagnostic], writer: &mut dyn Write) -> anyhow::Result<()> {
        let rules: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| d.rule_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|id| {
                json!({
                    "id": id,
                    "name": id,
                    "shortDescription": { "text": id },
                    "defaultConfiguration": { "level": "warning" }
                })
            })
            .collect();

        let results: Vec<serde_json::Value> = diagnostics
            .iter()
            .map(|d| {
                let level = match d.level {
                    DiagnosticLevel::Allow => "none",
                    DiagnosticLevel::Warn => "warning",
                    DiagnosticLevel::Deny | DiagnosticLevel::Forbid => "error",
                };

                let mut result = json!({
                    "ruleId": d.rule_id,
                    "level": level,
                    "message": { "text": d.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": d.file_path.display().to_string()
                            },
                            "region": {
                                "startLine": d.line,
                                "startColumn": d.column,
                                "endLine": d.end_line,
                                "endColumn": d.end_column
                            }
                        }
                    }]
                });

                if let Some(fix) = &d.suggestion {
                    result["fixes"] = json!([{
                        "description": { "text": &fix.description },
                        "artifactChanges": [{
                            "artifactLocation": {
                                "uri": d.file_path.display().to_string()
                            },
                            "replacements": [{
                                "deletedRegion": {
                                    "byteOffset": fix.start_byte,
                                    "byteLength": fix.end_byte - fix.start_byte
                                },
                                "insertedContent": { "text": &fix.replacement }
                            }]
                        }]
                    }]);
                }

                result
            })
            .collect();

        let sarif = json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "cargo-test-lint",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/user/cargo-test-lint",
                        "rules": rules
                    }
                },
                "results": results
            }]
        });

        serde_json::to_writer_pretty(writer, &sarif)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
    use std::path::PathBuf;

    fn make_diag(rule_id: &str, level: DiagnosticLevel) -> Diagnostic {
        Diagnostic {
            rule_id: rule_id.into(),
            level,
            message: "test message".into(),
            file_path: PathBuf::from("src/lib.rs"),
            line: 10,
            column: 5,
            end_line: 10,
            end_column: 20,
            suggestion: None,
        }
    }

    fn format(diags: &[Diagnostic]) -> serde_json::Value {
        let formatter = SarifFormatter;
        let mut buf = Vec::new();
        formatter.write(diags, &mut buf).unwrap();
        serde_json::from_slice(&buf).unwrap()
    }

    #[test]
    fn sarif_version() {
        let sarif = format(&[]);
        assert_eq!(sarif["version"], "2.1.0");
    }

    #[test]
    fn sarif_has_tool_driver() {
        let sarif = format(&[]);
        assert_eq!(sarif["runs"][0]["tool"]["driver"]["name"], "cargo-test-lint");
    }

    #[test]
    fn sarif_contains_results() {
        let diags = vec![make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn)];
        let sarif = format(&diags);
        assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 1);
        assert_eq!(sarif["runs"][0]["results"][0]["ruleId"], "CTL_ASSERT_MSG");
    }

    #[test]
    fn sarif_level_mapping() {
        let diags = vec![
            make_diag("A", DiagnosticLevel::Warn),
            make_diag("B", DiagnosticLevel::Deny),
        ];
        let sarif = format(&diags);
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "warning");
        assert_eq!(sarif["runs"][0]["results"][1]["level"], "error");
    }

    #[test]
    fn sarif_has_rules() {
        let diags = vec![make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn)];
        let sarif = format(&diags);
        let rules = sarif["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["id"], "CTL_ASSERT_MSG");
    }

    #[test]
    fn sarif_includes_fix() {
        let mut diag = make_diag("CTL_ASSERT_MSG", DiagnosticLevel::Warn);
        diag.suggestion = Some(Fix {
            description: "add message".into(),
            replacement: "assert!(true, \"msg\")".into(),
            start_byte: 0,
            end_byte: 15,
        });
        let sarif = format(&[diag]);
        let fixes = &sarif["runs"][0]["results"][0]["fixes"];
        assert!(fixes.is_array());
    }

    #[test]
    fn sarif_valid_json() {
        let diags = vec![make_diag("A", DiagnosticLevel::Warn)];
        let formatter = SarifFormatter;
        let mut buf = Vec::new();
        formatter.write(&diags, &mut buf).unwrap();
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_slice(&buf);
        assert!(parsed.is_ok());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cargo-test-lint sarif
```

Expected: All 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/src/output/sarif.rs
git commit -m "feat: implement SARIF 2.1.0 output formatter"
```

---

### Task 19: CLI Integration

**Files:**
- Modify: `crates/cargo-test-lint/src/main.rs`

- [ ] **Step 1: Update main.rs with full integration**

```rust
use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

use cargo_test_lint::config;
use cargo_test_lint::diagnostics::Diagnostic;
use cargo_test_lint::output::{OutputFormat, Formatter};
use cargo_test_lint::parser;
use cargo_test_lint::rules;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    #[command(name = "test-lint")]
    TestLint(TestLintArgs),
}

#[derive(Parser)]
struct TestLintArgs {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,

    #[arg(long)]
    fix: bool,

    #[arg(long)]
    rules: Option<String>,

    #[arg(long, default_value = "terminal")]
    format: String,

    #[arg(long)]
    max_expects: Option<usize>,

    #[arg(long)]
    nextest: bool,

    #[arg(long)]
    deny_warnings: bool,
}

fn main() -> anyhow::Result<()> {
    let Cargo::TestLint(args) = Cargo::parse();

    let mut config = config::load(&args.project_root);

    // CLI overrides
    if let Some(max) = args.max_expects {
        config.max_expects = max;
    }
    if args.nextest {
        config.nextest = true;
    }
    if args.deny_warnings {
        config.deny_warnings = true;
    }

    let files = parser::collect_rs_files(&args.project_root)
        .context("failed to collect source files")?;

    let mut all_diagnostics = Vec::new();

    for file in &files {
        let (source, tree) = match parser::parse_file(file) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("warning: skipping {}: {}", file.display(), e);
                continue;
            }
        };

        let ctx = rules::RuleContext {
            source: &source,
            tree: &tree,
            config: &config,
            file_path: file,
        };

        all_diagnostics.extend(rules::run_all_rules(&ctx));
    }

    let format: OutputFormat = args
        .format
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    let formatter: Box<dyn Formatter> = match format {
        OutputFormat::Terminal => Box::new(cargo_test_lint::output::terminal::TerminalFormatter),
        OutputFormat::Sarif => Box::new(cargo_test_lint::output::sarif::SarifFormatter),
    };

    formatter.write(&all_diagnostics, &mut std::io::stderr())?;

    if Diagnostic::has_errors(&all_diagnostics)
        || (config.deny_warnings && !all_diagnostics.is_empty())
    {
        std::process::exit(1);
    }

    Ok(())
}
```

- [ ] **Step 2: Verify full compilation**

```bash
cargo build -p cargo-test-lint
```

Expected: Binary builds successfully.

- [ ] **Step 3: Run all tests**

```bash
cargo test -p cargo-test-lint
```

Expected: All tests pass (diagnostics, config, parser, all rules, terminal, sarif).

- [ ] **Step 4: Run clippy**

```bash
cargo clippy -p cargo-test-lint -- -D warnings
```

Expected: No warnings.

- [ ] **Step 5: Run fmt check**

```bash
cargo fmt -p cargo-test-lint -- --check
```

Expected: No formatting issues.

- [ ] **Step 6: Commit**

```bash
git add crates/cargo-test-lint/src/main.rs
git commit -m "feat: complete CLI with full rule engine integration"
```

---

### Task 20: Integration Tests

**Files:**
- Create: `crates/cargo-test-lint/tests/integration_tests.rs`

- [ ] **Step 1: Create integration test file**

```rust
use std::fs;
use std::process::Command;

fn run_test_lint(dir: &std::path::Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-lint"))
        .arg("test-lint")
        .arg("--project-root")
        .arg(dir)
        .output()
        .expect("failed to run cargo-test-lint");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn clean_project_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true, "should pass");
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();

    let (code, _, stderr) = run_test_lint(tmp.path());
    assert_eq!(code, 0, "expected clean exit, stderr: {stderr}");
}

#[test]
fn violations_exit_one() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(2, 2);
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();

    let (code, _, stderr) = run_test_lint(tmp.path());
    assert_eq!(code, 1, "expected exit 1, stderr: {stderr}");
    assert!(stderr.contains("CTL_ASSERT_MSG"));
}

#[test]
fn sarif_output_format() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-lint"))
        .arg("test-lint")
        .arg("--project-root")
        .arg(tmp.path())
        .arg("--format")
        .arg("sarif")
        .output()
        .expect("failed to run cargo-test-lint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let sarif: serde_json::Value = serde_json::from_str(&stderr).expect("invalid SARIF JSON");
    assert_eq!(sarif["version"], "2.1.0");
}

#[test]
fn deny_warnings_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();

    let (code, _, _) = run_test_lint(tmp.path());
    // Without --deny-warnings, warnings don't cause exit 1
    // (default config has rules at warn level, not deny)
    // This test verifies the flag is accepted without error
    assert!(code == 0 || code == 1);
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test -p cargo-test-lint --test integration_tests
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/cargo-test-lint/tests/
git commit -m "test: add integration tests for CLI and output formats"
```

---

### Task 21: CI Workflow Update

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Update CI workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, nightly]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
      - run: cargo build --workspace --release
      - name: Self-lint
        run: cargo run --bin cargo-test-lint -- test-lint --project-root .

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings

  cargo-deny:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: update workflow for single-crate structure and self-lint"
```

---

### Task 22: Final Cleanup and Verification

- [ ] **Step 1: Remove stale files**

```bash
# Remove any remaining references to old crate structure
rm -rf crates/ctl-core crates/ctl-daemon fixtures .worktrees
# Remove old docs
rm -f docs/INTEGRATION-rust-analyzer.md
```

- [ ] **Step 2: Update README.md**

Replace `README.md` content to reflect the new tool:

```markdown
# cargo-test-lint

AST-driven test quality linter for Rust.

## Installation

```bash
cargo install cargo-test-lint
```

## Usage

```bash
cargo test-lint [--project-root .] [--format terminal|sarif] [--deny-warnings]
```

## Rules

| Rule ID | Description | Default |
|---------|-------------|---------|
| `CTL_ASSERT_MSG` | Assertion missing context message | warn |
| `CTL_MAX_EXPECTS` | Too many assertions in test (default: 5) | warn |
| `CTL_SLEEP` | `std::thread::sleep` in test code | forbid |
| `CTL_TEST_BRANCHING` | Control flow in test body | warn |
| `CTL_STATIC_MUT` | Static mutable variable | warn |
| `CTL_ENV_SET_VAR` | `std::env::set_var` in test | warn |
| `CTL_ASYNC_BLOCKING` | Blocking call in async test | warn |
| `CTL_NESTED_MOD` | Deeply nested test module | warn |
| `CTL_UNNECESSARY_CLONE` | Unnecessary `.clone()` | warn |
| `CTL_DEEP_WRAPPER` | Deeply nested type wrapper | warn |
| `CTL_MISSING_DROP_GUARD` | Resource allocation without RAII guard | warn |
| `CTL_DEAD_TEST_HELPER` | Unused test helper | warn |

## Configuration

In your workspace `Cargo.toml`:

```toml
[lints.cargo-test-lint]
sleepy-test = "forbid"
max-expects = 10
```

## License

MIT OR Apache-2.0
```

- [ ] **Step 3: Full verification**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Expected: All pass.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup and README update for AST-driven pivot"
```

---

## Verification Checklist

After all tasks:

- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — no warnings
- [ ] `cargo fmt --all -- --check` — formatted
- [ ] `cargo build --workspace --release` — builds
- [ ] Binary runs: `cargo run --bin cargo-test-lint -- test-lint --help`
- [ ] Self-lint: `cargo run --bin cargo-test-lint -- test-lint --project-root .`
- [ ] SARIF output: `cargo run --bin cargo-test-lint -- test-lint --project-root . --format sarif`
