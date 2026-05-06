use cargo_test_lint::config::Config;
use cargo_test_lint::diagnostics::DiagnosticLevel;
use proptest::collection::hash_map;
use proptest::prelude::*;

fn rule_key_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9-]{0,29}").unwrap()
}

prop_compose! {
    fn arb_diagnostic_level()(level in prop_oneof![
        Just(DiagnosticLevel::Allow),
        Just(DiagnosticLevel::Warn),
        Just(DiagnosticLevel::Deny),
        Just(DiagnosticLevel::Forbid),
    ]) -> DiagnosticLevel {
        level
    }
}

prop_compose! {
    fn arb_config()(
        rules in hash_map(
            rule_key_strategy(),
            arb_diagnostic_level(),
            0..10,
        ),
        max_expects in 0usize..100,
        max_nested_mod in 0usize..100,
        nextest in any::<bool>(),
        deny_warnings in any::<bool>(),
    ) -> Config {
        Config { rules, max_expects, max_nested_mod, nextest, deny_warnings }
    }
}

#[test]
fn default_config_roundtrip() {
    let config = Config::default();
    let toml_str = toml::to_string(&config).unwrap();
    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed, config);
}

proptest! {
    #[test]
    fn toml_roundtrip(config in arb_config()) {
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        prop_assert_eq!(parsed, config);
    }

    #[test]
    fn config_merge_commutative_non_overlapping(
        rules_a in hash_map(prop::string::string_regex("a-[a-z]{1,5}").unwrap(), arb_diagnostic_level(), 1..5),
        rules_b in hash_map(prop::string::string_regex("b-[a-z]{1,5}").unwrap(), arb_diagnostic_level(), 1..5),
        max_expects_a in 0usize..50,
        max_expects_b in 0usize..50,
        max_nested_mod_a in 0usize..50,
        max_nested_mod_b in 0usize..50,
    ) {
        let config_a = Config {
            rules: rules_a.clone(),
            max_expects: max_expects_a,
            max_nested_mod: max_nested_mod_a,
            nextest: false,
            deny_warnings: false,
        };
        let config_b = Config {
            rules: rules_b.clone(),
            max_expects: max_expects_b,
            max_nested_mod: max_nested_mod_b,
            nextest: true,
            deny_warnings: true,
        };

        let mut ab = config_a.clone();
        ab.rules.extend(config_b.rules.clone());
        ab.nextest = config_b.nextest;
        ab.deny_warnings = config_b.deny_warnings;

        let mut ba = config_b.clone();
        ba.rules.extend(config_a.rules.clone());
        ba.max_expects = config_a.max_expects;
        ba.max_nested_mod = config_a.max_nested_mod;

        prop_assert_eq!(ab, ba);
    }

    #[test]
    fn valid_toml_produces_valid_config(
        rules in hash_map(
            rule_key_strategy(),
            prop_oneof![Just("allow"), Just("warn"), Just("deny"), Just("forbid")],
            0..5,
        ),
        max_expects in 0usize..100,
        max_nested_mod in 0usize..100,
        nextest in any::<bool>(),
        deny_warnings in any::<bool>(),
    ) {
        let mut toml_str = format!(
            "max-expects = {max_expects}\n\
             max-nested-mod = {max_nested_mod}\n\
             nextest = {nextest}\n\
             deny-warnings = {deny_warnings}\n"
        );
        if !rules.is_empty() {
            toml_str.push_str("\n[rules]\n");
            for (k, v) in &rules {
                toml_str.push_str(&format!("{k} = \"{v}\"\n"));
            }
        }
        let result = toml::from_str::<Config>(&toml_str);
        prop_assert!(result.is_ok());
        let config = result.unwrap();
        prop_assert_eq!(config.max_expects, max_expects);
        prop_assert_eq!(config.max_nested_mod, max_nested_mod);
        prop_assert_eq!(config.nextest, nextest);
        prop_assert_eq!(config.deny_warnings, deny_warnings);
    }

    #[test]
    fn invalid_toml_returns_error(
        key in prop::string::string_regex("[a-z]{1,5}").unwrap(),
    ) {
        let invalid = format!("{key} = = =");
        let result = toml::from_str::<Config>(&invalid);
        prop_assert!(result.is_err());
    }

    #[test]
    fn malformed_field_types_never_panic(bad_value in prop_oneof![Just("true"), Just("\"not-a-number\""), Just("[1, 2]")]) {
        let toml_str = format!("max-expects = {bad_value}");
        let result = toml::from_str::<Config>(&toml_str);
        prop_assert!(result.is_err());
    }

    #[test]
    fn config_serialization_is_deterministic(config in arb_config()) {
        let first = toml::to_string(&config).unwrap();
        let second = toml::to_string(&config).unwrap();
        prop_assert_eq!(first, second);
    }
}
