# Lint Rule Epics — cargo-test-lint

Additional lint rules to expand coverage beyond the initial 12 rules (v0.2.0).

---

## Epic 1: Assertion Quality (v0.3.0)

**Goal:** Detect weak, missing, or tautological assertions.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_EMPTY_TEST` | Empty test | `warn` | Test function has no assertions. Flag `#[test]` functions with zero `assert*!` macro calls. |
| `CTL_TAUTOLOGICAL_ASSERT` | Tautological assertion | `warn` | `assert_eq!(true, expr)` or `assert_eq!(false, expr)` — use `assert!(expr)` / `assert!(!expr)` instead. |
| `CTL_SINGLE_ASSERT` | Single assertion | `info` | Test has exactly one assertion. May indicate incomplete coverage. Advisory only. |
| `CTL_ASSERT_WITHOUT_MESSAGE` | Assertion without message | `warn` | `assert!(x)` without message — hard to debug on failure. Recommend `assert!(x, "reason")`. |

### Implementation Notes

- `CTL_EMPTY_TEST`: Scan function body for `assert*!` macro invocations. Exclude functions with `#[should_panic]`.
- `CTL_TAUTOLOGICAL_ASSERT`: Pattern match `assert_eq!`/`assert_ne!` where one operand is `true`/`false`.
- `CTL_ASSERT_WITHOUT_MESSAGE`: Check if `assert!`/`assert_eq!` macro has 2+ args (first is condition, second is message).

---

## Epic 2: Unwrap Safety (v0.3.0)

**Goal:** Detect risky `unwrap()` / `expect()` usage in tests.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_UNWRAP_IN_TEST` | Unwrap in test | `warn` | `unwrap()` call inside test function. Use `?` with `Result<(), E>` return type or `.expect("context")`. |
| `CTL_EXPECT_LAZY` | Lazy expect message | `info` | `expect("static string")` — consider `expect(&format!("context: {:?}", val))` for better diagnostics. |

### Implementation Notes

- `CTL_UNWRAP_IN_TEST`: Find all `.unwrap()` calls in test function bodies. Exclude `expect()` (separate rule). Consider allowlist for known-safe types (e.g., `Mutex::lock`).
- `CTL_EXPECT_LAZY`: Find `expect()` calls with static string literals. Advisory — suggest format strings for better error context.

---

## Epic 3: Test Isolation (v0.4.0)

**Goal:** Detect tests that share mutable state or have cleanup issues.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_ENV_SET_NO_CLEANUP` | Env set without cleanup | `warn` | `std::env::set_var` without corresponding `remove_var` or RAII guard. |
| `CTL_FILE_WRITE_NO_CLEANUP` | File write without cleanup | `warn` | `File::create` / `fs::write` in test without `tempfile` or cleanup in drop. |
| `CTL_GLOBAL_MUTEX` | Global mutex in test | `warn` | `Mutex::new(())` at module level used for test serialization. Indicates flaky test design. |

### Implementation Notes

- `CTL_ENV_SET_NO_CLEANUP`: Track `set_var` calls. Check if `remove_var` or `tempfile::env::var` guard exists in same scope.
- `CTL_FILE_WRITE_NO_CLEANUP`: Track `File::create`/`fs::write` calls. Check if path uses `tempfile` or has explicit cleanup.
- `CTL_GLOBAL_MUTEX`: Find `static.*Mutex` declarations. Advisory — suggest `#[serial]` or test isolation.

---

## Epic 4: Performance (v0.4.0)

**Goal:** Detect slow or wasteful test patterns.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_LARGE_LOOP` | Large loop in test | `warn` | Loop with `> 10_000` iterations in test body. May indicate performance issue. |
| `CTL_ALLOC_IN_LOOP` | Allocation in loop | `warn` | `Vec::new()` / `String::new()` inside loop body. Consider pre-allocating. |
| `CTL_TEST_TIMEOUT` | Missing timeout | `info` | Async test without `#[timeout]` attribute. Advisory for CI stability. |

### Implementation Notes

- `CTL_LARGE_LOOP`: Find `for`/`while`/`loop` with literal iteration counts > threshold (configurable).
- `CTL_ALLOC_IN_LOOP`: Track heap allocations inside loop bodies. Advisory — suggest `with_capacity` or pre-allocation.
- `CTL_TEST_TIMEOUT`: Check for `#[tokio::test]` without `#[timeout]`. Advisory for async tests.

---

## Epic 5: Flakiness Detection (v0.5.0)

**Goal:** Detect patterns that cause intermittent test failures.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_TIME_DEPENDENT` | Time-dependent assertion | `warn` | `SystemTime::now()` or `Instant::now()` used in assertion. May cause flaky tests. |
| `CTL_RANDOM_SEED` | Unseeded randomness | `warn` | `rand::random()` or `rand::thread_rng()` without fixed seed. Non-deterministic. |
| `CTL_SLEEP_SYNC` | Sleep for sync | `warn` | `thread::sleep` used to wait for async condition. Use `tokio::time::timeout` or retry loop. |
| `CTL_ORDER_DEPENDENT` | Order-dependent test | `warn` | Test modifies shared state that another test reads. Detect via static/global mutation analysis. |

### Implementation Notes

- `CTL_TIME_DEPENDENT`: Track `SystemTime::now()`/`Instant::now()` usage near `assert*!` calls.
- `CTL_RANDOM_SEED`: Find `rand::random()`/`thread_rng()` without `seed()` or `StdRng::seed()`.
- `CTL_SLEEP_SYNC`: `thread::sleep` in async context or without timeout wrapper.
- `CTL_ORDER_DEPENDENT`: Cross-function analysis of static/global mutations. Complex — defer to v0.5.0.

---

## Epic 6: Error Handling Coverage (v0.5.0)

**Goal:** Ensure tests cover both success and failure paths.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_NO_ERROR_PATH` | Missing error path test | `warn` | Function returns `Result` but no test covers the `Err` case. |
| `CTL_SHOULD_PANIC_MSG` | Should panic without message | `warn` | `#[should_panic]` without `expected` parameter. Too broad — may catch unrelated panics. |

### Implementation Notes

- `CTL_NO_ERROR_PATH`: Analyze function signatures returning `Result`. Check if any test exercises error conditions. Requires cross-function analysis.
- `CTL_SHOULD_PANIC_MSG`: Check `#[should_panic]` attribute for `expected` parameter.

---

## Epic 7: Test Naming & Structure (v0.6.0)

**Goal:** Enforce naming conventions and structural best practices.

### Rules

| ID | Name | Severity | Description |
|----|------|----------|-------------|
| `CTL_TEST_NAME_VAGUE` | Vague test name | `warn` | Test named `test_*` without descriptive suffix. E.g., `test_it_works`, `test_basic`. |
| `CTL_GIVEN_WHEN_THEN` | Missing GWT structure | `info` | Test body without `// Arrange`, `// Act`, `// Assert` comments or equivalent structure. Advisory. |
| `CTL_TOO_MANY_ASSERTS` | Too many assertions | `warn` | Test has `> 5` assertions. May be testing too many things. Consider splitting. |

### Implementation Notes

- `CTL_TEST_NAME_VAGUE`: Pattern match test function names against vague patterns: `test_it_*`, `test_basic`, `test_simple`, `test_works`.
- `CTL_GIVEN_WHEN_THEN`: Check for `// Given`, `// When`, `// Then` or `// Arrange`, `// Act`, `// Assert` comments. Advisory.
- `CTL_TOO_MANY_ASSERTS`: Count `assert*!` invocations per function. Configurable threshold.

---

## Priority Summary

| Epic | Version | Rules | Complexity | Impact |
|------|---------|-------|------------|--------|
| 1. Assertion Quality | v0.3.0 | 4 | Low | High |
| 2. Unwrap Safety | v0.3.0 | 2 | Low | High |
| 3. Test Isolation | v0.4.0 | 3 | Medium | High |
| 4. Performance | v0.4.0 | 3 | Medium | Medium |
| 5. Flakiness Detection | v0.5.0 | 4 | High | High |
| 6. Error Handling | v0.5.0 | 2 | High | Medium |
| 7. Naming & Structure | v0.6.0 | 3 | Low | Medium |

**Total new rules:** 21 (from 12 to 33)

---

## Implementation Order

1. **v0.3.0** — Epics 1+2 (6 rules, low complexity, high impact)
2. **v0.4.0** — Epics 3+4 (6 rules, medium complexity)
3. **v0.5.0** — Epics 5+6 (6 rules, high complexity — cross-function analysis)
4. **v0.6.0** — Epic 7 (3 rules, naming conventions)
