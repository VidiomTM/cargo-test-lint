# ADR-005: Testing Strategy

**Status:** Accepted

**Context:** A lint tool for test quality must itself be thoroughly tested. Rules must correctly identify violations without false positives. The AST-based approach means tests can use inline Rust source strings rather than real files.

**Decision:** Use standard Rust testing with `#[test]` functions. Each rule has tests for: positive cases (should flag violation), negative cases (should not flag), and edge cases (empty test, async test, nested modules). Use `proptest` for property-based testing of rule robustness. Integration tests run the CLI against temporary directories with known test files.

**Consequences:**
- Positive: Inline source strings make tests self-contained and fast
- Positive: Property-based testing catches edge cases in AST traversal
- Positive: Integration tests verify full pipeline (config → parse → lint → output)
- Negative: Inline strings may drift from real Rust syntax patterns
- Negative: Large test suite for each rule (10+ test cases per rule)

**Alternatives:**
- Snapshot testing (insta): Better for output format verification but heavier
- File-based test fixtures: Slower, harder to read in test code
