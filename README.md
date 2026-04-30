# cargo-test-lint

Unified Rust test telemetry linter. Surfaces cargo-llvm-cov coverage gaps and cargo-mutants surviving mutants as rustc-style JSON diagnostics consumed by rust-analyzer via `check.overrideCommand`.

## Quick Start

1. Install: `cargo install cargo-test-lint`
2. Configure rust-analyzer: set `check.overrideCommand` to `cargo-test-lint`
3. Diagnostics appear inline as you edit

## Architecture

Three crates: `ctl-core` (types), `ctl-daemon` (async file watcher + pipeline), `ctl` (CLI for rust-analyzer). See [ARCHITECTURE.md](ARCHITECTURE.md).

## License

MIT OR Apache-2.0
