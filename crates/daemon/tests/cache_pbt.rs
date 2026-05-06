use ctl_daemon::cache::Cache;
use proptest::collection::vec;
use proptest::prelude::*;
use std::time::{Duration, Instant};

proptest! {
    #[test]
    fn get_after_set_returns_cached_value(
        key in "[a-zA-Z][a-zA-Z0-9_-]{0,49}",
        value in vec(any::<u8>(), 0..1000),
        ttl_ms in 1u64..10000,
        max_size in 1usize..64,
    ) {
        let mut cache = Cache::new(max_size);
        let now = Instant::now();
        let ttl = Duration::from_millis(ttl_ms);
        cache.set_at(key.clone(), value.clone(), now, ttl);
        let result = cache.get_at(&key, now);
        prop_assert_eq!(result, Some(value));
    }

    #[test]
    fn eviction_reduces_size_to_max(
        max_size in 1usize..16,
        num_entries in 1usize..100,
    ) {
        let mut cache = Cache::new(max_size);
        let now = Instant::now();
        for i in 0..num_entries {
            cache.set_at(
                format!("key-{i}"),
                vec![i as u8],
                now,
                Duration::from_secs(60),
            );
        }
        cache.evict_at(now);
        prop_assert!(cache.len() <= max_size);
    }

    #[test]
    fn ttl_expiry_returns_none_after_expiry(
        key in "[a-zA-Z][a-zA-Z0-9_-]{0,49}",
        value in vec(any::<u8>(), 0..100),
        ttl_ms in 1u64..1000,
        after_ms in 1001u64..10000,
    ) {
        let mut cache = Cache::new(16);
        let now = Instant::now();
        let ttl = Duration::from_millis(ttl_ms);
        cache.set_at(key.clone(), value, now, ttl);
        let future = now + Duration::from_millis(after_ms);
        let result = cache.get_at(&key, future);
        prop_assert_eq!(result, None);
    }

    #[test]
    fn overwrite_returns_latest_value(
        key in "[a-zA-Z][a-zA-Z0-9_-]{0,49}",
        first_value in vec(any::<u8>(), 0..100),
        second_value in vec(any::<u8>(), 0..100),
        ttl_ms in 1u64..10000,
    ) {
        let mut cache = Cache::new(16);
        let now = Instant::now();
        let ttl = Duration::from_millis(ttl_ms);
        cache.set_at(key.clone(), first_value, now, ttl);
        cache.set_at(key.clone(), second_value.clone(), now + ttl, ttl);
        let result = cache.get_at(&key, now + ttl);
        prop_assert_eq!(result, Some(second_value));
    }

    #[test]
    fn empty_cache_returns_none(
        key in "[a-zA-Z][a-zA-Z0-9_-]{0,49}",
    ) {
        let cache = Cache::new(16);
        let result = cache.get(&key);
        prop_assert_eq!(result, None);
    }

    #[test]
    fn cache_size_never_exceeds_max_size(
        max_size in 1usize..32,
        operations in vec(
            (0u64..1000u64, vec(any::<u8>(), 0..50)),
            0..200,
        ),
    ) {
        let mut cache = Cache::new(max_size);
        let base = Instant::now();
        for (i, (delay_ms, value)) in operations.iter().enumerate() {
            let now = base + Duration::from_millis(*delay_ms);
            cache.set_at(
                format!("key-{i}"),
                value.clone(),
                now,
                Duration::from_secs(60),
            );
        }
        cache.evict_at(base + Duration::from_millis(1000));
        prop_assert!(cache.len() <= max_size);
    }

    #[test]
    fn get_after_overwrite_then_evict_returns_correct_value(
        key in "[a-zA-Z][a-zA-Z0-9_-]{0,49}",
        value1 in vec(any::<u8>(), 0..100),
        value2 in vec(any::<u8>(), 0..100),
        value3 in vec(any::<u8>(), 0..100),
    ) {
        let mut cache = Cache::new(8);
        let now = Instant::now();
        let ttl = Duration::from_secs(10);
        cache.set_at(key.clone(), value1, now, ttl);
        cache.set_at(key.clone(), value2.clone(), now + Duration::from_secs(1), ttl);
        cache.set_at(key.clone(), value3.clone(), now + Duration::from_secs(2), ttl);
        let result = cache.get_at(&key, now + Duration::from_secs(3));
        prop_assert_eq!(result, Some(value3));
    }
}
