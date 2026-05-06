mod common;

use common::strategies::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn smoke_proptest_setup(_ in proptest::bool::ANY) {
        prop_assert!(true);
    }
    #[test]
    fn arb_coverage_line_generates_valid_data(coverage in arb_coverage_line()) {
        let (file, start, end, _count) = coverage;
        prop_assert!(file.ends_with(".rs"));
        prop_assert!(start <= end);
    }
    #[test]
    fn arb_span_generates_valid_spans(span in arb_span()) {
        let (start_line, start_col, end_line, end_col) = span;
        prop_assert!(start_line <= end_line);
        if start_line == end_line {
            prop_assert!(start_col <= end_col);
        }
    }
    #[test]
    fn arb_mutation_generates_valid_data(mutation in arb_mutation()) {
        let (file, line, status) = mutation;
        prop_assert!(file.ends_with(".rs"));
        prop_assert!(line > 0);
        let valid = ["missed", "caught", "timeout", "build-failure", "unreachable"];
        prop_assert!(valid.contains(&status.as_str()));
    }
    #[test]
    fn arb_config_generates_valid_toml(config in arb_config()) {
        prop_assert!(config.contains("[rules]"));
        prop_assert!(config.contains("max-expects"));
        prop_assert!(config.contains("max-nested-mod"));
    }
}
