# Rule Reference

All 15 rules in cargo-test-lint v0.2.0.

## Assertion Roulette

**ID:** `assertion-roulette`  
**Default:** `warn`  
**Category:** Assertions

`assert!`/`assert_eq!`/`assert_ne!` without a context message. Assertions without messages are hard to debug on failure.

### Bad

```rust
#[test]
fn test_addition() {
    let result = add(2, 3);
    assert_eq!(result, 5);
    assert!(result > 0);
}
```

### Good

```rust
#[test]
fn test_addition() {
    let result = add(2, 3);
    assert_eq!(result, 5, "2 + 3 should equal 5");
    assert!(result > 0, "result should be positive");
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
assertion-roulette = "deny"
```

---

## Max Expects

**ID:** `max-expects`  
**Default:** `warn`  
**Category:** Assertions

Test has too many assertions (default threshold: 5). May indicate the test is testing too many things.

### Bad

```rust
#[test]
fn test_user() {
    let user = create_user("Alice");
    assert_eq!(user.name, "Alice");
    assert_eq!(user.age, 30);
    assert!(user.is_active);
    assert_eq!(user.role, "admin");
    assert!(user.email.is_some());
    assert_eq!(user.created_at.year(), 2026);
}
```

### Good

```rust
#[test]
fn test_user_name() {
    let user = create_user("Alice");
    assert_eq!(user.name, "Alice", "should set name");
}

#[test]
fn test_user_role() {
    let user = create_user("Alice");
    assert_eq!(user.role, "admin", "should default to admin");
}
```

### Config

```toml
[lints.cargo-test-lint]
max-expects = 10  # default: 5, 0 disables
```

---

## Sleepy Test

**ID:** `sleepy-test`  
**Default:** `forbid`  
**Category:** Flow

`std::thread::sleep` in test code. Tests should be fast and deterministic.

### Bad

```rust
#[test]
fn test_eventually_ready() {
    start_background_task();
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(is_ready());
}
```

### Good

```rust
#[test]
fn test_eventually_ready() {
    start_background_task();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !is_ready() {
        if std::time::Instant::now() > deadline {
            panic!("task not ready within timeout");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
sleepy-test = "forbid"  # cannot be overridden
```

---

## Test Branching

**ID:** `test-branching`  
**Default:** `warn`  
**Category:** Flow

`if`/`match` in test body. Tests should be deterministic — branching indicates conditional logic that may hide failures.

### Bad

```rust
#[test]
fn test_config() {
    let config = load_config();
    if config.debug {
        assert_eq!(config.log_level, "debug");
    } else {
        assert_eq!(config.log_level, "info");
    }
}
```

### Good

```rust
#[test]
fn test_debug_config() {
    let config = load_config_debug();
    assert_eq!(config.log_level, "debug", "debug config should have debug log level");
}

#[test]
fn test_release_config() {
    let config = load_config_release();
    assert_eq!(config.log_level, "info", "release config should have info log level");
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
test-branching = "allow"  # suppress
```

---

## Async Blocking

**ID:** `async-blocking`  
**Default:** `warn`  
**Category:** Async Safety

Blocking call in `#[tokio::test]`. Blocking the async runtime can cause deadlocks and slow tests.

### Bad

```rust
#[tokio::test]
async fn test_fetch() {
    let response = reqwest::blocking::get("https://example.com").unwrap();
    assert!(response.status().is_success());
}
```

### Good

```rust
#[tokio::test]
async fn test_fetch() {
    let response = reqwest::get("https://example.com").await.unwrap();
    assert!(response.status().is_success());
}
```

### Detected Patterns

- `std::thread::sleep` in async test
- `reqwest::blocking::*` in async test
- `std::fs::*` in async test
- `std::io::*` in async test

### Config

```toml
[lints.cargo-test-lint.rules]
async-blocking = "deny"
```

---

## Nested Mod

**ID:** `nested-mod`  
**Default:** `warn`  
**Category:** Structure

Deeply nested test module (default max depth: 3). Deep nesting makes tests hard to find and maintain.

### Bad

```rust
#[cfg(test)]
mod tests {
    mod user {
        mod creation {
            mod validation {
                #[test]
                fn test_email() { /* ... */ }
            }
        }
    }
}
```

### Good

```rust
#[cfg(test)]
mod tests {
    mod user_creation {
        #[test]
        fn test_email_validation() { /* ... */ }
    }
}
```

### Config

```toml
[lints.cargo-test-lint]
max-nested-mod = 2  # default: 3, 0 disables
```

---

## Unnecessary Clone

**ID:** `unnecessary-clone`  
**Default:** `warn`  
**Category:** Cloning

`.clone()` on value that isn't used after the clone. Wastes allocations.

### Bad

```rust
#[test]
fn test_name() {
    let name = String::from("Alice");
    let cloned = name.clone();
    assert_eq!(cloned, "Alice");
    // `name` never used again
}
```

### Good

```rust
#[test]
fn test_name() {
    let name = String::from("Alice");
    assert_eq!(name, "Alice");
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
unnecessary-clone = "allow"
```

---

## Deep Wrapper

**ID:** `deep-wrapper`  
**Default:** `warn`  
**Category:** Complexity

Type wrapper nested >3 levels deep. Hard to understand and maintain.

### Bad

```rust
struct Wrapper(Result<Option<Vec<String>>, Error>);
// 4 levels deep: Wrapper > Result > Option > Vec > String
```

### Good

```rust
type MaybeItems = Option<Vec<String>>;
type ItemsResult = Result<MaybeItems, Error>;
struct Wrapper(ItemsResult);
```

### Config

```toml
[lints.cargo-test-lint.rules]
deep-wrapper = "allow"
```

---

## Dead Test Helper

**ID:** `dead-test-helper`  
**Default:** `warn`  
**Category:** Dead Code

Unused function/struct in test module. Dead code clutters test suites.

### Bad

```rust
#[cfg(test)]
mod tests {
    fn helper_old() -> i32 { 42 }  // never called
    
    #[test]
    fn test_something() {
        assert_eq!(1 + 1, 2);
    }
}
```

### Good

```rust
#[cfg(test)]
mod tests {
    fn helper() -> i32 { 42 }
    
    #[test]
    fn test_something() {
        assert_eq!(helper(), 42);
    }
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
dead-test-helper = "allow"
```

---

## Static Mut

**ID:** `static-mut`  
**Default:** `warn`  
**Category:** Nextest Compatibility

`static mut` variable. Incompatible with nextest which runs tests in separate processes.

### Bad

```rust
static mut COUNTER: i32 = 0;

#[test]
fn test_counter() {
    unsafe {
        COUNTER += 1;
        assert_eq!(COUNTER, 1);
    }
}
```

### Good

```rust
use std::sync::atomic::{AtomicI32, Ordering};

static COUNTER: AtomicI32 = AtomicI32::new(0);

#[test]
fn test_counter() {
    COUNTER.fetch_add(1, Ordering::SeqCst);
    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
}
```

### Config

```toml
[lints.cargo-test-lint]
nextest = true  # enable nextest checks

[lints.cargo-test-lint.rules]
static-mut = "deny"
```

---

## Env Set Var

**ID:** `env-set-var`  
**Default:** `warn`  
**Category:** Nextest Compatibility

`std::env::set_var` in test. Unsafe with nextest which runs tests in parallel processes.

### Bad

```rust
#[test]
fn test_config() {
    std::env::set_var("MY_CONFIG", "test");
    let config = load_config();
    assert_eq!(config.value, "test");
}
```

### Good

```rust
#[test]
fn test_config() {
    // Use tempfile::env or pass config directly
    let config = load_config_with_env("MY_CONFIG", "test");
    assert_eq!(config.value, "test");
}
```

### Config

```toml
[lints.cargo-test-lint]
nextest = true

[lints.cargo-test-lint.rules]
env-set-var = "deny"
```

---

## Missing Drop Guard

**ID:** `missing-drop-guard`  
**Default:** `warn`  
**Category:** Resource Safety

Resource-allocating call (`TempDir::new`, `tempfile::tempdir`, `NamedTempFile::new`) not bound to a variable. The resource will be dropped immediately, causing test flakiness.

### Bad

```rust
#[test]
fn test_file() {
    TempDir::new().unwrap();  // dropped immediately!
    write_file("data.txt");    // dir may already be gone
}
```

### Good

```rust
#[test]
fn test_file() {
    let dir = TempDir::new().unwrap();  // bound — lives until end of scope
    write_file(dir.path().join("data.txt"));
}
```

### Detected Patterns

- `TempDir::new()`, `tempfile::tempdir()`, `NamedTempFile::new()` as standalone expressions or passed as arguments without being bound to a `let` binding.

### Config

```toml
[lints.cargo-test-lint.rules]
missing-drop-guard = "deny"
```

---

## String Literal Corpus

**ID:** `string-literal-corpus`  
**Default:** `warn`  
**Category:** Semantic

Test corpus code (Python, JS, etc.) embedded in a Rust string literal inside a test function. Embedded code is invisible to the Rust AST parser and should live in separate fixture files.

### Bad

```rust
#[test]
fn test_parse() {
    let input = r#"
import pytest
from myapp import Calculator

def test_addition():
    calc = Calculator()
    assert calc.add(2, 3) == 5
"#;
    let ast = parse(input);
    assert_eq!(ast.len(), 3);
}
```

### Good

```rust
#[test]
fn test_parse() {
    let input = include_str!("../fixtures/calculator_test.py");
    let ast = parse(input);
    assert_eq!(ast.len(), 3);
}
```

### Detected Signals

- `def test_`, `it(`, `describe(`, `import pytest`, `from pytest`, `from vitest`, `import vitest` inside string literals ≥40 chars within test context.

### Config

```toml
[lints.cargo-test-lint.rules]
string-literal-corpus = "allow"  # suppress if fixture approach not feasible
```

---

## FS IO in Test

**ID:** `fs-io-in-test`  
**Default:** `warn`  
**Category:** Semantic

Direct `std::fs::*` / `fs::*` calls inside test functions. These introduce filesystem flakiness. Prefer `tempfile` APIs or in-memory I/O.

### Bad

```rust
#[test]
fn test_config() {
    std::fs::write("config.toml", "key = value").unwrap();
    let cfg = load_config("config.toml");
    assert_eq!(cfg.key, "value");
    // file leaked to working directory!
}
```

### Good

```rust
#[test]
fn test_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.toml"), "key = value").unwrap();
    let cfg = load_config(dir.path().join("config.toml"));
    assert_eq!(cfg.key, "value");
}
```

### Detected Patterns

- `std::fs::write`, `std::fs::read`, `std::fs::read_to_string`, `std::fs::remove_file`, `std::fs::create_dir`, `std::fs::remove_dir`, `std::fs::rename`, and their short-form `fs::*` equivalents.

### Config

```toml
[lints.cargo-test-lint.rules]
fs-io-in-test = "allow"
```
