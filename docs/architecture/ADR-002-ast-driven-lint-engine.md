---
id: ADR-002
kind: adr
title: "AST-Driven Lint Engine with Tree-sitter"
status: draft
authors: []
reviewers: []
tags: []
supersedes: []
superseded_by: []
depends_on: []
blocks: []
implements: []
related: []
external: []
project: cargo-test-lint
---

**Context:** The previous architecture wrapped `cargo-llvm-cov` and `cargo-mutants` via an async daemon with IPC. This required external tools, had high latency, and could not detect structural test quality issues like assertion roulette, sleepy tests, or dead helper functions.

**Decision:** Use Tree-sitter (via `tree-sitter` + `tree-sitter-rust` crates) for Rust source code parsing. Rules use a hybrid query+validation approach: Tree-sitter queries match patterns (e.g., `thread::sleep` calls), then Rust validation logic checks context and emits diagnostics. The rule engine is a trait-based registry for extensibility.

**Consequences:**
- Positive: Fast, standalone parsing without external tool dependencies
- Positive: Tree-sitter queries are composable and reusable across rules
- Positive: Can detect structural anti-patterns invisible to coverage tools
- Negative: Only supports languages with Tree-sitter grammars
- Negative: Queries may miss dynamically constructed patterns (e.g., `sleep(dur)` where `dur` is a variable)

**Alternatives:**
- Regex-based scanning: Faster but misses AST structure (nested blocks, function scope)
- Syn/venial (proc-macro parsing): Full Rust AST but slower, harder to query
