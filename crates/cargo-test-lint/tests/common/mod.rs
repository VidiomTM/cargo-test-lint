pub mod strategies;

use proptest::test_runner::Config as ProptestConfig;

#[allow(dead_code)]
pub fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(256),
        max_shrink_iters: std::env::var("PROPTEST_MAX_SHRINK_ITERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4096),
        ..ProptestConfig::default()
    }
}
