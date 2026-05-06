use proptest::prelude::*;

pub fn arb_coverage_line() -> impl Strategy<Value = (String, usize, usize, usize)> {
    (
        "[a-z_][a-z0-9_]*\\.rs",
        (1usize..1000),
        (1usize..1000),
        (0usize..1000),
    )
        .prop_map(|(file, s, e, count)| {
            let start = s.min(e);
            let end = s.max(e);
            (format!("src/{file}"), start, end, count)
        })
}

pub fn arb_span() -> impl Strategy<Value = (usize, usize, usize, usize)> {
    (
        (1usize..500),
        (1usize..120),
        (1usize..500),
        (1usize..120),
    )
        .prop_map(|(sl, sc, el, ec)| {
            let (start_line, end_line) = if sl <= el { (sl, el) } else { (el, sl) };
            let (start_col, end_col) = if start_line == end_line {
                if sc <= ec { (sc, ec) } else { (ec, sc) }
            } else {
                (sc, ec)
            };
            (start_line, start_col, end_line, end_col)
        })
}

pub fn arb_mutation() -> impl Strategy<Value = (String, usize, String)> {
    (
        "[a-z_][a-z0-9_]*\\.rs",
        (1usize..1000),
        prop_oneof![
            Just("missed"),
            Just("caught"),
            Just("timeout"),
            Just("build-failure"),
            Just("unreachable"),
        ],
    )
        .prop_map(|(file, line, status)| {
            (format!("src/{file}"), line, status.to_string())
        })
}

pub fn arb_config() -> impl Strategy<Value = String> {
    (
        "[a-z_][a-z0-9_]*",
        prop_oneof![Just("allow"), Just("warn"), Just("deny"), Just("forbid")],
        (0usize..20),
        (0usize..10),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(rule_id, level, max_expects, max_nested_mod, nextest, deny_warnings)| {
            format!(
                "max-expects = {max_expects}\n\
                 max-nested-mod = {max_nested_mod}\n\
                 nextest = {nextest}\n\
                 deny-warnings = {deny_warnings}\n\
                 \n\
                 [rules]\n\
                 {rule_id} = \"{level}\"\n"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::proptest;

    proptest! {
        #[test]
        fn coverage_line_is_valid(coverage in arb_coverage_line()) {
            let (file, start, end, _count) = coverage;
            assert!(file.ends_with(".rs"));
            assert!(start <= end);
        }
        #[test]
        fn span_is_valid(span in arb_span()) {
            let (sl, sc, el, ec) = span;
            assert!(sl <= el);
            if sl == el { assert!(sc <= ec); }
        }
        #[test]
        fn mutation_is_valid(mutation in arb_mutation()) {
            let (file, line, status) = mutation;
            assert!(file.ends_with(".rs"));
            assert!(line > 0);
            let valid = ["missed", "caught", "timeout", "build-failure", "unreachable"];
            assert!(valid.contains(&status.as_str()));
        }
        #[test]
        fn config_contains_rules(config in arb_config()) {
            assert!(config.contains("[rules]"));
            assert!(config.contains("max-expects"));
            assert!(config.contains("max-nested-mod"));
        }
    }
}
