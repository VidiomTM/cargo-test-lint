use ctl_core::config::Config;

#[test]
fn config_default_values() {
    let config = Config::default();
    assert!(config.coverage.enabled);
    assert_eq!(config.coverage.timeout_secs, 300);
    assert!(config.coverage.extra_args.is_empty());
    assert!(config.mutation.enabled);
    assert_eq!(config.mutation.timeout_secs, 600);
    assert!(config.mutation.filter_uncovered);
    assert!(config.mutation.extra_args.is_empty());
    assert_eq!(config.daemon.debounce_ms, 500);
    assert_eq!(config.daemon.full_sweep_interval_secs, 300);
    assert!(config.daemon.socket_path.is_none());
    assert_eq!(config.output.level, "warning");
    assert!(config.output.show_diff_hunks);
}
