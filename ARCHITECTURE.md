# Architecture

## Components

- **ctl-core**: Shared types — CoverageGap, SurvivingMutant, Diagnostic (rustc format), Span, Config
- **ctl-daemon**: Async daemon — file watcher (notify), subprocess runners (cargo-llvm-cov, cargo-mutants), CoverageMatrix, NDJSON cache, IPC server (UDS/named pipe), pipeline orchestration
- **ctl**: CLI binary — clap arg parser, daemon lifecycle management, streams CompilerMessage JSON to stdout

## Data Flow

1. File watcher detects .rs change
2. Daemon runs coverage (cargo-llvm-cov) and mutation (cargo-mutants) analysis
3. Results filtered through CoverageMatrix (skip mutants on uncovered lines)
4. Cached as NDJSON in target/ctl-cache/
5. CLI (triggered by rust-analyzer) reads cache, streams rustc-format diagnostics to stdout

## Key Types

- `CoverageGap` → `Diagnostic` via `coverage_to_diagnostics()`
- `SurvivingMutant` → `Diagnostic` via `mutant_to_diagnostics()`
- `CoverageMatrix` — maps files to covered/uncovered lines
