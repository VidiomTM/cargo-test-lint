use proptest::prelude::*;

pub fn arb_coverage_line() -> impl Strategy<Value = (String, usize, usize, u32)> {
    let file_regex = prop::string::string_regex("[a-zA-Z0-9_/]+\\.rs").unwrap();
    (file_regex, 1usize..10_000, 1usize..10_000, 0u32..1_000).prop_map(
        |(file, start, end, count)| {
            let end = std::cmp::max(start, end);
            (file, start, end, count)
        },
    )
}

pub fn arb_span() -> impl Strategy<Value = (usize, usize, usize, usize)> {
    (1usize..10_000, 0usize..200, 1usize..10_000, 0usize..200).prop_map(|(sl, sc, el, ec)| {
        let (el, ec) =
            if sl > el || (sl == el && sc > ec) { (sl, std::cmp::max(sc, ec)) } else { (el, ec) };
        (sl, sc, el, ec)
    })
}

pub fn arb_mutation() -> impl Strategy<Value = (String, usize, String)> {
    let file_regex = prop::string::string_regex("[a-zA-Z0-9_/]+\\.rs").unwrap();
    (file_regex, 1usize..10_000, prop_oneof!["killed", "survived", "timeout"])
        .prop_map(|(file, line, status)| (file, line, status.to_string()))
}

pub fn arb_config() -> impl Strategy<Value = String> {
    let rule_regex = prop::string::string_regex("[a-z][a-z0-9-]{0,29}").unwrap();
    (
        proptest::collection::vec(rule_regex, 0..10),
        0usize..100,
        0usize..100,
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(rules, max_expects, max_nested_mod, nextest, deny_warnings)| {
            let mut toml_str = format!(
                "max-expects = {max_expects}\n\
                 max-nested-mod = {max_nested_mod}\n\
                 nextest = {nextest}\n\
                 deny-warnings = {deny_warnings}\n"
            );
            if !rules.is_empty() {
                toml_str.push_str("\n[rules]\n");
                for rule in &rules {
                    toml_str.push_str(&format!("{rule} = \"warn\"\n"));
                }
            }
            toml_str
        })
}
