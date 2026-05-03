# cargo-test-lint

AST-driven test quality linter for Rust. Catches common test anti-patterns at compile time using tree-sitter parsing.

## Quick Start

```bash
cargo install cargo-test-lint
cargo test-lint
```

## Rules

| Rule | ID | Default | Description |
|------|-----|---------|-------------|
| Assertion Roulette | `assertion-roulette` | warn | `assert!`/`assert_eq!`/`assert_ne!` without context message |
| Max Expects | `max-expects` | warn | Test has too many assertions (default threshold: 5) |
| Sleepy Test | `sleepy-test` | forbid | `std::thread::sleep` in test code |
| Test Branching | `test-branching` | warn | `if`/`match` in test body (tests should be deterministic) |
| Async Blocking | `async-blocking` | warn | Blocking call in `#[tokio::test]` |
| Nested Mod | `nested-mod` | warn | Deeply nested test module (default max depth: 3) |
| Unnecessary Clone | `unnecessary-clone` | warn | `.clone()` on value that isn't reused |
| Deep Wrapper | `deep-wrapper` | warn | Type wrapper nested >3 levels deep |
| Missing Drop Guard | `missing-drop-guard` | warn | Resource allocation without RAII binding |
| Dead Test Helper | `dead-test-helper` | warn | Unused function/struct in test module |
| Static Mut | `static-mut` | warn | `static mut` variable (incompatible with nextest) |
| Env Set Var | `env-set-var` | warn | `std::env::set_var` in test (unsafe with nextest) |
| String Literal Corpus | `string-literal-corpus` | warn | Test corpus code embedded in string literals |
| Filesystem IO | `fs-io-in-test` | warn | `std::fs` calls in test (flakiness) |

## Configuration

Add to `Cargo.toml` (workspace or package level):

```toml
[lints.cargo-test-lint]
max-expects = 10
max-nested-mod = 2
deny-warnings = true

[lints.cargo-test-lint.rules]
sleepy-test = "deny"
test-branching = "allow"
```

### Options

- `max-expects` — Max assertions per test before warning (0 disables, default: 5)
- `max-nested-mod` — Max nesting depth for test modules (0 disables, default: 3)
- `nextest` — Enable nextest-specific checks
- `deny-warnings` — Exit with error if any warnings found
- `rules` — Per-rule level overrides: `allow`, `warn`, `deny`, `forbid`

### CLI Flags

```
cargo test-lint [OPTIONS]

Options:
  --project-root <PATH>  Project root [default: .]
  --fix                  Auto-fix where possible
  --rules <RULES>        Filter rules
  --format <FORMAT>      Output format: terminal, sarif [default: terminal]
  --max-expects <N>      Override max assertions threshold
  --nextest              Enable nextest checks
  --deny-warnings        Treat warnings as errors
```

## Output Formats

**Terminal** (default) — Colored diagnostics for local development.

**SARIF** — Static Analysis Results Interchange Format for CI and tool integration.

## IDE Integration

### rust-analyzer

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "cargo test-lint"
}
```

Diagnostics appear inline as you edit.

## Architecture

Single crate using tree-sitter for AST parsing. See [ARCHITECTURE.md](ARCHITECTURE.md).

## License

MIT OR Apache-2.0
