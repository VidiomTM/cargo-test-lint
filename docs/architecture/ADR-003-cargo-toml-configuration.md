# ADR-003: Configuration via Cargo.toml

**Status:** Accepted

**Context:** Rust RFC 3389 defines a standard `[lints]` table in Cargo.toml for tool configuration. Using this makes the linter feel native to the Rust ecosystem. Users configure test lint rules alongside clippy and rustc lints.

**Decision:** Parse configuration from `[lints.cargo-test-lint]` table in Cargo.toml using the `toml` crate. Configuration supports enabling/disabling rules, setting severity levels, and per-rule options (e.g., max assertion count). Falls back to sensible defaults if not configured.

**Consequences:**
- Positive: Zero additional config files — integrates with existing Cargo.toml
- Positive: Follows Rust convention (RFC 3389)
- Positive: Workspace-level lint config propagates to all members
- Negative: Only works for Cargo-managed projects
- Negative: Config changes require Cargo.toml edit (may trigger unnecessary rebuilds)

**Alternatives:**
- Separate config file (`.test-lint.toml`): Additional file, not RFC 3389 compliant
- CLI flags only: Impractical for project-level configuration
