use proptest::prelude::*;

pub fn arb_coverage_line() -> impl Strategy<Value = (String, usize, usize, usize)> {
    let file_regex = prop::string::string_regex("[a-z_][a-z0-9_]*\\.rs").unwrap();
    (file_regex, (1usize..1000), (1usize..1000), (0usize..1000)).prop_map(|(file, s, e, count)| {
        let start = s.min(e);
        let end = s.max(e);
        (format!("src/{file}"), start, end, count)
    })
}

pub fn arb_span() -> impl Strategy<Value = (usize, usize, usize, usize)> {
    ((1usize..500), (1usize..120), (1usize..500), (1usize..120)).prop_map(|(sl, sc, el, ec)| {
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
    let file_regex = prop::string::string_regex("[a-z_][a-z0-9_]*\\.rs").unwrap();
    (
        file_regex,
        (1usize..1000),
        prop_oneof![
            Just("missed"),
            Just("caught"),
            Just("timeout"),
            Just("build-failure"),
            Just("unreachable"),
        ],
    )
        .prop_map(|(file, line, status)| (format!("src/{file}"), line, status.to_string()))
}

pub fn arb_config() -> impl Strategy<Value = String> {
    let rule_regex = prop::string::string_regex("[a-z_][a-z0-9_]*").unwrap();
    (
        rule_regex,
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
