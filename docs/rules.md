# Rule Reference

All 12 rules in cargo-test-lint v0.2.0.

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

## Missing Drop Guard

**ID:** `missing-drop-guard`  
**Default:** `warn`  
**Category:** Drop

Resource allocation without RAII binding. Resources may leak if test panics.

### Bad

```rust
#[test]
fn test_file() {
    let path = "/tmp/test.txt";
    std::fs::write(path, "hello").unwrap();
    // file not cleaned up if assertion below panics
    let content = std::fs::read_to_string(path).unwrap();
    assert_eq!(content, "hello");
    std::fs::remove_file(path).unwrap();
}
```

### Good

```rust
#[test]
fn test_file() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "hello").unwrap();
    let content = std::fs::read_to_string(file.path()).unwrap();
    assert_eq!(content, "hello");
    // file automatically cleaned up
}
```

### Config

```toml
[lints.cargo-test-lint.rules]
missing-drop-guard = "deny"
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
