# ADR-004: Output Formats (SARIF + Terminal)

**Status:** Accepted

**Context:** Lint diagnostics must be human-readable in the terminal for development and machine-readable for CI/CD integration. Different consumption contexts require different output formats.

**Decision:** Support two output formats: colored terminal output (default, using `anstyle` crate) and SARIF 2.1.0 (JSON, for GitHub Actions and CI integration). SARIF output includes rule metadata, file locations, and severity levels compatible with GitHub's code scanning.

**Consequences:**
- Positive: Terminal output is immediately actionable for developers
- Positive: SARIF integrates with GitHub code scanning and VS Code
- Positive: anstyle is lightweight (no external terminal UI dependency)
- Negative: SARIF output requires transformation to be displayed inline in PRs
- Negative: Two output paths means more code to maintain

**Alternatives:**
- JSON only: Integrates everywhere but no human-friendly output
- Terminal only: No CI integration without additional tooling
- clippy-style output: Familiar but not machine-readable
