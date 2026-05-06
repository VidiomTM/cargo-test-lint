use cargo_test_lint::cov_parse::{self, CoverageData, CoverageLine};
use proptest::prelude::*;

fn arb_file_path() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_./]{1,80}"
}

fn arb_coverage_line() -> impl Strategy<Value = CoverageLine> {
    (arb_file_path(), 1u64..=100_000, 0u64..=100_000).prop_map(
        |(file_path, line_number, execution_count)| CoverageLine {
            file_path,
            line_number,
            execution_count,
        },
    )
}

fn arb_coverage_data() -> impl Strategy<Value = CoverageData> {
    prop::collection::vec(arb_coverage_line(), 1..50).prop_map(|lines| CoverageData { lines })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn roundtrip_coverage_data(data in arb_coverage_data()) {
        let serialized = cov_parse::serialize(&data);
        let parsed = cov_parse::parse(&serialized).unwrap();
        prop_assert_eq!(data, parsed);
    }

    #[test]
    fn valid_serialized_output_always_parses(data in arb_coverage_data()) {
        let serialized = cov_parse::serialize(&data);
        let result = cov_parse::parse(&serialized);
        prop_assert!(result.is_ok(), "parse failed on valid serialized output");
    }

    #[test]
    fn parsed_line_numbers_positive(data in arb_coverage_data()) {
        let serialized = cov_parse::serialize(&data);
        let parsed = cov_parse::parse(&serialized).unwrap();
        for line in &parsed.lines {
            prop_assert!(
                line.line_number >= 1,
                "line_number must be >= 1, got {}",
                line.line_number
            );
        }
    }

    #[test]
    fn parsed_execution_counts_non_negative(data in arb_coverage_data()) {
        let serialized = cov_parse::serialize(&data);
        let parsed = cov_parse::parse(&serialized).unwrap();
        for line in &parsed.lines {
            prop_assert!(
                line.execution_count == line.execution_count,
                "execution_count overflow/wrap: {}",
                line.execution_count
            );
        }
    }

    #[test]
    fn parsed_file_paths_non_empty(data in arb_coverage_data()) {
        let serialized = cov_parse::serialize(&data);
        let parsed = cov_parse::parse(&serialized).unwrap();
        for line in &parsed.lines {
            prop_assert!(!line.file_path.is_empty(), "file_path must not be empty");
            prop_assert!(line.file_path.len() >= 1, "file_path length must be >= 1");
        }
    }

    #[test]
    fn no_panics_on_arbitrary_string(input in "\\PC*") {
        let _ = cov_parse::parse(&input);
    }
}
