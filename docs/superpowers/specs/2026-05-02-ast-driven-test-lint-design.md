# AST-Driven Test Quality Linter — Design Spec

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create the implementation plan from this spec.

**Goal:** Pivot `cargo-test-lint` from a coverage/mutation wrapper daemon into a standalone, AST-driven test quality linter powered by Tree-sitter.

**Architecture:** Single Rust crate using Tree-sitter for parsing with a hybrid query+validation rule engine. Rules are configured via `Cargo.toml [lints]` table per RFC 3389. Output in SARIF 2.1.0 and colored terminal formats.

**Tech Stack:** Rust (edition 2024, MSRV 1.85), tree-sitter + tree-sitter-rust, clap, serde, serde_json, anstyle

---

## 1. Problem Statement

The current tool wraps `cargo-llvm-cov` and `cargo-mutants` via an async daemon with IPC, file watching, and NDJSON caching. This architecture:

- Requires external tools (`cargo-llvm-cov`, `cargo-mutants`) installed
- Has high latency (subprocess spawning, coverage matrix computation)
- Cannot detect structural test quality issues (assertion roulette, sleepy tests, dead helpers)
- Couples tightly to nightly compiler internals via coverage instrumentation

The pivot replaces this with a fast, standalone AST-driven linter that detects test quality antipatterns directly from source code.

---

## 2. Architecture

### 2.1 Module Structure

```text
src/
├── main.rs              # CLI entry point (clap)
├── lib.rs               # Module re-exports
├── config.rs            # Cargo.toml [lints.cargo-test-lint] parser
├── parser.rs            # Tree-sitter setup, source file loading
├── rules/
│   ├── mod.rs           # Rule trait, registry, execution engine
│   ├── nextest.rs       # --nextest-compatibility rules
│   ├── async_safety.rs  # Async runtime safety checks
│   ├── assertions.rs    # Assertion roulette, assertion count
│   ├── flow.rs          # Deterministic flow control
│   ├── sleep.rs         # Sleepy test forbiddance
│   ├── structure.rs     # Structural thresholds (max-expects, nesting)
│   ├── cloning.rs       # Clone-everything heuristic
│   ├── complexity.rs    # Architectural complexity warnings
│   ├── drop.rs          # Drop semantics enforcement
│   └── dead_code.rs     # Dead code in #[cfg(test)] modules
├── diagnostics.rs       # Diagnostic types
└── output/
    ├── mod.rs           # Output trait, format selection
    ├── sarif.rs         # SARIF 2.1.0 formatter
    └── terminal.rs      # Colored terminal formatter
```

### 2.2 Data Flow

1. CLI parses args + loads config from `Cargo.toml [lints.cargo-test-lint]`
2. Parser loads `.rs` files via Tree-sitter (walk workspace with `ignore` crate for gitignore support)
3. Rule engine runs enabled rules on each file:
   - Each rule defines a Tree-sitter query to find candidate nodes
   - Query matches are passed to `validate()` which applies precise logic
   - Validated violations become `Diagnostic` objects
4. Diagnostics collected, sorted by file position
5. Output formatter renders diagnostics (SARIF or terminal)
6. Exit code: 0 = clean, 1 = violations, 2 = runtime error

### 2.3 Parsing Engine: Hybrid Approach

**Tree-sitter queries** (S-expression) for candidate discovery:
- Cheap, declarative pattern matching across the AST
- Example: find all `assert!` macro calls, all `#[test]` functions, all `.clone()` calls

**Custom Rust validation** for precise checking:
- Count arguments, check types, build reference graphs, compute thresholds
- Applied only to query-matched candidates (not full tree traversal)

This mirrors how production linters (semgrep, ast-grep, clippy) work.

---

## 3. Rule Engine

### 3.1 Rule Trait

```rust
pub trait Rule {
    /// Rule identifier (e.g., "CTL_ASYNC_BLOCKING")
    fn id(&self) -> &'static str;

    /// Human-readable description
    fn description(&self) -> &'static str;

    /// Default severity level
    fn default_level(&self) -> DiagnosticLevel;

    /// Tree-sitter query string to find candidate nodes
    fn query(&self) -> &'static str;

    /// Validate a candidate match and emit zero or more diagnostics
    fn validate(&self, ctx: &RuleContext, match: &QueryMatch) -> Vec<Diagnostic>;
}
```

### 3.2 Rule Context

```rust
pub struct RuleContext<'a> {
    pub source: &'a [u8],          // Raw source bytes
    pub tree: &'a Tree,            // Parsed Tree-sitter tree
    pub config: &'a RuleConfig,    // Per-rule config values
    pub file_path: &'a Path,       // File being linted
}
```

### 3.3 Execution Engine

For each source file:
1. Parse with Tree-sitter → `Tree`
2. For each enabled rule:
   a. Run `rule.query()` → `QueryCursor` → collect `QueryMatch` candidates
   b. For each match, call `rule.validate(ctx, &match)` → collect diagnostics
3. Merge diagnostics from all rules
4. Sort by (file, line, column)

### 3.4 Rule Configuration

Parsed from workspace `Cargo.toml`:

```toml
[lints.cargo-test-lint]
# Boolean rules: allow / warn / deny / forbid
nextest-compatibility = "warn"
async-blocking = "error"
assertion-roulette = "warn"
sleepy-test = "forbid"
test-branching = "warn"
unnecessary-clone = "warn"
deep-wrapper = "warn"
missing-drop-guard = "warn"
dead-test-helper = "warn"

# Threshold rules: numeric value
max-expects = 5
max-nested-mod = 3
```

Default: all rules `warn`, thresholds as specified above.

---

## 4. Rule Catalog

### 4.1 `CTL_STATIC_MUT` / `CTL_ENV_SET_VAR` / `CTL_FILE_LOCK` (nextest compatibility)

**Query:** `static mut` items, `std::env::set_var` calls, file lock acquisitions
**Validation:** Flag any usage in test code. These break process-isolated test runners.
**Level:** Configurable via `nextest-compatibility`

### 4.2 `CTL_ASYNC_BLOCKING` (async runtime safety)

**Query:** `#[tokio::test]` or `#[async_std::test]` attributed functions
**Validation:** Walk function body for blocking calls: `std::fs::*`, `std::thread::sleep`, `std::net::*`, `std::io::*` synchronous methods
**Level:** Configurable via `async-blocking`

### 4.3 `CTL_ASSERT_MSG` (assertion roulette)

**Query:** `assert!`, `assert_eq!`, `assert_ne!` macro invocations
**Validation:** Check if a format string argument is present (3rd+ arg for `assert!`, 3rd+ for `assert_eq!`/`assert_ne!`)
**Message:** `"assertion missing context message — add a format string for readable CI failures"`

### 4.4 `CTL_TEST_BRANCHING` (deterministic flow control)

**Query:** `if_expression`, `match_expression`, `loop_expression`, `while_expression` nodes inside `#[test]` function bodies
**Validation:** Flag control flow constructs that could cause assertions to be silently skipped
**Message:** `"conditional/loop in test body — assertions may be bypassed silently"`

### 4.5 `CTL_SLEEP` (sleepy test)

**Query:** `std::thread::sleep` call expressions
**Validation:** Always flag. Suggest `tokio::time::sleep`, `Condvar`, or channels.
**Level:** `forbid` by default

### 4.6 `CTL_MAX_EXPECTS` (assertion count threshold)

**Query:** `#[test]` function bodies
**Validation:** Count `assert*!` macro calls within the function body. Flag if count exceeds configured threshold (default: 5).
**Message:** `"test has {count} assertions (max {threshold}) — consider splitting"`

### 4.7 `CTL_NESTED_MOD` (module nesting depth)

**Query:** `mod` items with `#[cfg(test)]` attribute or named `tests`
**Validation:** Count nesting depth of test modules. Flag if exceeds threshold (default: 3).
**Message:** `"test module nesting depth {depth} (max {threshold}) — flatten structure"`

### 4.8 `CTL_UNNECESSARY_CLONE` (clone heuristic)

**Query:** `.clone()` call expressions
**Validation:** Check if the cloned value is used again after the clone. If not, suggest borrowing (`&T`) or `AsRef`.
**Message:** `"value cloned but not reused — consider borrowing instead"`

### 4.9 `CTL_DEEP_WRAPPER` (architectural complexity)

**Query:** Type annotations in test setup code
**Validation:** Count nesting depth of generic wrappers (`Arc<Mutex<Option<T>>>`). Flag if depth exceeds 3.
**Message:** `"deeply nested type wrapper ({depth} levels) — test setup is overly complex"`

### 4.10 `CTL_MISSING_DROP_GUARD` (drop semantics)

**Query:** Resource allocation calls in test functions (`File::create`, `TcpListener::bind`, `TempDir::new`, etc.)
**Validation:** Check if the allocated resource is bound to a variable that is explicitly dropped or goes out of scope naturally (RAII). Flag if resource is allocated without binding.
**Message:** `"resource allocation without RAII guard — may leak on assertion panic"`

### 4.11 `CTL_DEAD_TEST_HELPER` (dead code in test modules)

**Query:** `fn`, `struct`, `trait`, `type` definitions inside `#[cfg(test)]` modules
**Validation:** Build a reference map within the test module scope. Definitions with zero references (excluding the definition itself) are flagged.
**Scope:** Single-crate, within `#[cfg(test)]` blocks only. No cross-crate analysis.
**Message:** `"unused test helper '{name}' — defined but never referenced"`

---

## 5. Diagnostic Types

```rust
pub struct Diagnostic {
    pub rule_id: String,           // e.g., "CTL_ASSERT_MSG"
    pub level: DiagnosticLevel,    // allow / warn / deny / forbid
    pub message: String,           // Human-readable message
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub suggestion: Option<Fix>,   // Auto-fix suggestion
}

pub struct Fix {
    pub description: String,
    pub replacement: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

pub enum DiagnosticLevel {
    Allow,
    Warn,
    Deny,
    Forbid,
}
```

---

## 6. Output Formats

### 6.1 Terminal (default)

Colored, human-readable output:

```text
warning[CTL_ASSERT_MSG]: assertion missing context message
  --> src/lib.rs:42:9
   |
42 |         assert_eq!(result, 42);
   |         ^^^^^^^^^^^^^^^^^^^^^^ help: add a message: `assert_eq!(result, 42, "expected 42")`
```

Uses `anstyle` for cross-platform color support.

### 6.2 SARIF 2.1.0

Standard JSON format for CI integration:

```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [{
    "tool": {
      "driver": {
        "name": "cargo-test-lint",
        "version": "0.1.0",
        "rules": [...]
      }
    },
    "results": [...]
  }]
}
```

Compatible with GitHub Actions Code Scanning, VS Code SARIF viewer.

---

## 7. CLI Interface

```text
cargo test-lint [OPTIONS]

Options:
  --project-root <PATH>     Project root (default: current dir)
  --fix                     Auto-fix where possible
  --rules <RULES>           Comma-separated rule IDs to run (default: all enabled)
  --format <FORMAT>         Output format: terminal (default), sarif
  --max-expects <N>         Override max-expects threshold
  --nextest                 Enable nextest-compatibility rules
  --deny-warnings           Exit 1 on any warning (not just errors)
  -h, --help                Print help
  -V, --version             Print version
```

Works as cargo subcommand via `cargo_` prefix convention.

---

## 8. Dependencies

### Add
- `tree-sitter` (AST parsing)
- `tree-sitter-rust` (Rust grammar)
- `anstyle` (terminal colors)
- `ignore` (gitignore-aware file walking) — already in workspace

### Remove
- `tokio` (async runtime — no longer needed)
- `notify` (file watcher — no longer needed)
- `libc` (process management — no longer needed)
- `tracing` / `tracing-subscriber` (daemon logging — no longer needed)

### Keep
- `clap` (CLI)
- `serde` / `serde_json` (config, SARIF output)
- `toml` (Cargo.toml parsing)
- `thiserror` / `anyhow` (error handling)
- `ignore` (file walking)
- `tempfile` (testing)

---

## 9. Migration Plan (What Gets Removed)

### Strip entirely
- `ctl-daemon` crate (IPC, file watcher, pipeline, cache, coverage/mutation runners)
- `ctl` crate's daemon management (PID file, signal handler, socket path)
- `ctl-core`'s `coverage.rs`, `mutation.rs`, `coverage_to_diagnostics()`, `mutant_to_diagnostics()`
- All fixtures (coverage/mutation test projects)
- `.worktrees/` directory
- `docs/INTEGRATION-rust-analyzer.md`

### Keep/repurpose
- `ctl-core`'s `config.rs` → becomes `config.rs` (adapted for `[lints]` table)
- `ctl-core`'s `diagnostic.rs` → becomes `diagnostics.rs` (adapted for SARIF)
- `ctl-core`'s `span.rs` → still useful for byte offset calculations
- `ctl`'s `main.rs` → becomes new CLI entry point
- CI workflows (adapted to run `cargo test-lint` instead of daemon)
- `deny.toml`, `clippy.toml`, `rustfmt.toml`

### Workspace Cargo.toml changes
- Remove `ctl-core`, `ctl-daemon`, `ctl` from `members`
- Add single `cargo-test-lint` crate (or rename `ctl`)
- Update dependencies as specified in Section 8

---

## 10. Testing Strategy

### Unit tests per rule
Each rule module gets its own `#[cfg(test)]` module with:
- Test helper that parses a Rust snippet and runs the specific rule
- Positive tests (violations detected)
- Negative tests (clean code passes)

### Integration tests
- `tests/` directory with fixture `.rs` files containing known violations
- Run `cargo test-lint` against fixtures, assert expected diagnostics
- Test SARIF output structure
- Test terminal output formatting
- Test config loading from `Cargo.toml`

### Test helper macro
```rust
macro_rules! assert_lint {
    ($rule:expr, $source:expr, $expected:expr) => {
        let ctx = parse_source($source);
        let diagnostics = run_rule($rule, &ctx);
        assert_eq!(diagnostics.len(), $expected);
    };
}
```

---

## 11. Success Criteria

1. `cargo test-lint` runs on any Rust workspace without external tool dependencies
2. All 11 rule categories implemented and tested
3. Configuration via `Cargo.toml [lints.cargo-test-lint]` works
4. SARIF output validates against schema
5. Terminal output is colored and readable
6. Exit codes: 0 (clean), 1 (violations), 2 (errors)
7. `--fix` auto-fixes at least `CTL_ASSERT_MSG` (add format string)
8. CI workflow updated to run `cargo test-lint`
9. No external tool dependencies (no `cargo-llvm-cov`, `cargo-mutants`)
10. Performance: lints a 10k LOC workspace in < 2 seconds
