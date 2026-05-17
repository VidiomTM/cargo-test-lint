# AGENTS.md — cargo-test-lint

## Build & Test Commands

```bash
cargo build --workspace
cargo test --workspace -- --include-ignored
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

## PR Instructions

- Branch: feature/*, fix/*, chore/*
- Title: `<type>(<scope>): <description>`
- Types: feat, fix, docs, style, refactor, perf, test, build, ci, chore
- Run `cargo fmt` + `cargo clippy` before committing
- One logical change per commit
