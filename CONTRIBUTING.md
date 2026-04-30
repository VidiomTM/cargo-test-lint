# Contributing

## Setup

```bash
cargo build --workspace
cargo test --workspace
```

## Code Style

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

## PR Process

1. Create feature branch from main
2. Write tests first
3. Implement
4. Ensure CI passes
5. Open PR with conventional commit messages

## Commit Convention

Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
