# User Guide

## Installation

### From crates.io

```bash
cargo install cargo-test-lint
```

### From source

```bash
git clone https://github.com/Jonathangadeaharder/cargo-test-lint
cd cargo-test-lint
cargo install --path crates/cargo-test-lint
```

## Usage

### Basic

```bash
# Lint current directory
cargo test-lint

# Lint specific project
cargo test-lint --project-root /path/to/project
```

### With cargo

The binary is named `cargo-test-lint` so cargo recognizes it as a subcommand:

```bash
cargo test-lint
cargo test-lint --format sarif > results.sarif
```

### CLI Options

```
cargo test-lint [OPTIONS]

Options:
  --project-root <PATH>    Project root directory [default: .]
  --fix                    Auto-fix where possible
  --rules <RULES>          Comma-separated rule filter (e.g., "assertion-roulette,sleepy-test")
  --format <FORMAT>        Output format: terminal (default), sarif
  --max-expects <N>        Override max assertions threshold
  --max-nested-mod <N>     Override max nesting depth
  --nextest                Enable nextest-specific checks
  --deny-warnings          Exit with error code if any warnings found
  -h, --help               Print help
  -V, --version            Print version
```

## Configuration

### Cargo.toml (recommended)

Configure per RFC 3389 in `Cargo.toml`. Supports workspace and package level.

**Package level:**

```toml
[lints.cargo-test-lint]
# Threshold options
max-expects = 10
max-nested-mod = 2

# Global flags
deny-warnings = true
nextest = true

# Per-rule overrides
[lints.cargo-test-lint.rules]
sleepy-test = "deny"
test-branching = "allow"
assertion-roulette = "warn"
```

**Workspace level** (in root `Cargo.toml`):

```toml
[workspace.lints.cargo-test-lint]
max-expects = 5
deny-warnings = true

[workspace.lints.cargo-test-lint.rules]
sleepy-test = "forbid"
```

Package-level settings override workspace-level.

### Rule Levels

| Level | Behavior |
|-------|----------|
| `allow` | Suppress diagnostic |
| `warn` | Show warning, continue |
| `deny` | Show error, exit non-zero |
| `forbid` | Error, cannot be overridden |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CARGO_TEST_LINT_MAX_EXPECTS` | Override max-expects threshold |
| `CARGO_TEST_LINT_MAX_NESTED_MOD` | Override max-nested-mod threshold |

## Output Formats

### Terminal (default)

Colored output for local development:

```
warning[assertion-roulette]: assertion without message
  --> src/lib.rs:15:9
   |
15 |         assert_eq!(result, 42);
   |         ^^^^^^^^^^^^^^^^^^^^^^ help: add a message: `assert_eq!(result, 42, "expected 42")`
```

### SARIF

Static Analysis Results Interchange Format for CI integration:

```bash
cargo test-lint --format sarif > results.sarif
```

Compatible with GitHub Code Scanning, VS Code SARIF Viewer, and other SARIF consumers.

## IDE Integration

### rust-analyzer (VS Code)

Add to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "cargo test-lint"
}
```

Diagnostics appear inline as you edit.

### Neovim (LSP)

With `rust-tools.nvim` or `rustaceanvim`:

```lua
vim.g.rustaceanvim = {
  tools = {
    crate_graph = {
      backend = "dot",
    },
  },
  server = {
    default_settings = {
      ["rust-analyzer"] = {
        check = {
          command = "cargo test-lint",
        },
      },
    },
  },
}
```

## CI Integration

### GitHub Actions

```yaml
- name: Run cargo-test-lint
  run: |
    cargo install cargo-test-lint
    cargo test-lint --format sarif > results.sarif
    
- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

See [ci.md](ci.md) for more examples.

## Troubleshooting

### "no test functions found"

The linter looks for `#[test]` functions. If your tests are in a separate crate or use a custom test harness, they may not be detected.

### "tree-sitter parse error"

If the linter can't parse your file, check for syntax errors. The linter uses tree-sitter which handles most valid Rust, but may struggle with very new syntax features.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | No issues or all issues allowed |
| 1 | Warnings found (with `--deny-warnings`) |
| 2 | Errors found |
| 3 | Parse/config error |
