# CI Integration

## GitHub Actions

### Basic

```yaml
name: Lint
on: [push, pull_request]

jobs:
  test-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-test-lint
        run: cargo install cargo-test-lint
      
      - name: Run linter
        run: cargo test-lint --deny-warnings
```

### With SARIF upload

```yaml
name: Lint
on: [push, pull_request]

jobs:
  test-lint:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install cargo-test-lint
        run: cargo install cargo-test-lint
      
      - name: Run linter
        run: cargo test-lint --format sarif > results.sarif
      
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

### With caching

```yaml
name: Lint
on: [push, pull_request]

jobs:
  test-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/cargo-test-lint
            ~/.cargo/registry/index
            ~/.cargo/registry/cache
            target
          key: ${{ runner.os }}-cargo-test-lint-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Install cargo-test-lint
        run: |
          if ! command -v cargo-test-lint &> /dev/null; then
            cargo install cargo-test-lint
          fi
      
      - name: Run linter
        run: cargo test-lint --deny-warnings
```

### Multi-toolchain

```yaml
name: Lint
on: [push, pull_request]

jobs:
  test-lint:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, nightly]
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust ${{ matrix.rust }}
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      
      - name: Install cargo-test-lint
        run: cargo install cargo-test-lint
      
      - name: Run linter
        run: cargo test-lint --deny-warnings
```

## GitLab CI

```yaml
test-lint:
  image: rust:latest
  script:
    - cargo install cargo-test-lint
    - cargo test-lint --format sarif > results.sarif
  artifacts:
    reports:
      sast: results.sarif
```

## CircleCI

```yaml
version: 2.1
jobs:
  test-lint:
    docker:
      - image: cimg/rust:1.85
    steps:
      - checkout
      - run:
          name: Install cargo-test-lint
          command: cargo install cargo-test-lint
      - run:
          name: Run linter
          command: cargo test-lint --deny-warnings

workflows:
  lint:
    jobs:
      - test-lint
```

## Pre-commit hook

`.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-test-lint
        name: cargo-test-lint
        entry: cargo test-lint --deny-warnings
        language: system
        files: '\.rs$'
        pass_filenames: false
```

## VS Code

`.vscode/settings.json`:

```json
{
  "rust-analyzer.check.command": "cargo test-lint",
  "rust-analyzer.check.extraArgs": ["--deny-warnings"]
}
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Clean — no diagnostics |
| 1 | Warnings found (with `--deny-warnings`) |
| 2 | Errors found |
| 3 | Configuration or parse error |

Use `--deny-warnings` in CI to fail on any diagnostic.
