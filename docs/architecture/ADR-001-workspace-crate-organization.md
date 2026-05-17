---
id: ADR-001
kind: adr
title: "Workspace Structure and Crate Organization"
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

**Context:** The project has pivoted from a coverage/mutation wrapper daemon to an AST-driven test quality linter. The architecture needs clear separation between the CLI entry point, the core lint engine, and the optional daemon process.

**Decision:** Use a Cargo workspace with three crates: `crates/cargo-test-lint` (CLI binary, clap-based), `crates/core` (lint rule engine, Tree-sitter parsing, config), and `crates/daemon` (optional background file watcher). Core has no binary — it is a library consumed by the other two.

**Consequences:**
- Positive: Clear dependency direction (CLI → Core, Daemon → Core)
- Positive: Core can be used independently as a library
- Positive: Daemon can be removed without affecting the CLI
- Negative: Workspace overhead for a project where most changes target one crate
- Negative: Cross-crate refactoring requires workspace-wide coordination

**Alternatives:**
- Single crate with feature flags: Works but blurs separation of concerns
- Separate repositories: Too much overhead for tightly coupled crates
