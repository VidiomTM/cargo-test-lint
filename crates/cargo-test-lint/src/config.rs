use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::diagnostics::DiagnosticLevel;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case", default)]
pub struct Config {
    pub rules: HashMap<String, DiagnosticLevel>,
    pub max_expects: usize,
    pub max_nested_mod: usize,
    pub nextest: bool,
    pub deny_warnings: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("assertion-roulette".into(), DiagnosticLevel::Warn);
        rules.insert("sleepy-test".into(), DiagnosticLevel::Forbid);
        rules.insert("test-branching".into(), DiagnosticLevel::Warn);
        rules.insert("async-blocking".into(), DiagnosticLevel::Warn);
        rules.insert("unnecessary-clone".into(), DiagnosticLevel::Warn);
        rules.insert("deep-wrapper".into(), DiagnosticLevel::Warn);
        rules.insert("missing-drop-guard".into(), DiagnosticLevel::Warn);
        rules.insert("dead-test-helper".into(), DiagnosticLevel::Warn);
        rules.insert("nextest-compatibility".into(), DiagnosticLevel::Warn);
        rules.insert("string-literal-corpus".into(), DiagnosticLevel::Warn);
        rules.insert("fs-io-in-test".into(), DiagnosticLevel::Warn);

        Self { rules, max_expects: 5, max_nested_mod: 3, nextest: false, deny_warnings: false }
    }
}

impl Config {
    pub fn rule_level(&self, rule_id: &str, default: DiagnosticLevel) -> DiagnosticLevel {
        self.rules.get(rule_id).cloned().unwrap_or(default)
    }

    pub fn rule_enabled(&self, rule_id: &str) -> bool {
        !matches!(self.rules.get(rule_id), Some(DiagnosticLevel::Allow))
    }
}

pub fn load(project_root: &Path) -> Config {
    let cargo_toml = project_root.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&cargo_toml) else {
        return Config::default();
    };

    #[derive(Deserialize)]
    struct Manifest {
        workspace: Option<Workspace>,
        lints: Option<Lints>,
    }

    #[derive(Deserialize)]
    struct Workspace {
        lints: Option<Lints>,
    }

    #[derive(Deserialize)]
    struct Lints {
        #[serde(rename = "cargo-test-lint")]
        cargo_test_lint: Option<Config>,
    }

    // Package-level lints take precedence over workspace-level (RFC 3389)
    if let Ok(manifest) = toml::from_str::<Manifest>(&content) {
        if let Some(lints) = manifest.lints {
            if let Some(config) = lints.cargo_test_lint {
                return config;
            }
        }
        if let Some(ws) = manifest.workspace {
            if let Some(lints) = ws.lints {
                if let Some(config) = lints.cargo_test_lint {
                    return config;
                }
            }
        }
    }

    Config::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_cargo_toml(dir: &std::path::Path, content: &str) {
        let path = dir.join("Cargo.toml");
        let tmp = tempfile::NamedTempFile::new_in(dir).unwrap();
        std::io::Write::write_all(&mut tmp.as_file(), content.as_bytes()).unwrap();
        tmp.persist(path).unwrap();
    }

    #[test]
    fn default_config_has_all_rules_enabled() {
        let config = Config::default();
        assert!(
            config.rule_enabled("assertion-roulette"),
            "assertion-roulette should be enabled by default"
        );
        assert!(config.rule_enabled("sleepy-test"), "sleepy-test should be enabled by default");
        assert!(
            config.rule_enabled("test-branching"),
            "test-branching should be enabled by default"
        );
        assert_eq!(config.max_expects, 5, "default max_expects should be 5");
        assert_eq!(config.max_nested_mod, 3, "default max_nested_mod should be 3");
    }

    #[test]
    fn default_config_nextest_is_false() {
        let config = Config::default();
        assert!(!config.nextest, "nextest should be false by default");
    }

    #[test]
    fn rule_level_returns_default_when_not_configured() {
        let config = Config::default();
        assert_eq!(
            config.rule_level("nonexistent", DiagnosticLevel::Deny),
            DiagnosticLevel::Deny,
            "unconfigured rule should return default level"
        );
    }

    #[test]
    fn rule_enabled_false_when_allowed() {
        let mut config = Config::default();
        config.rules.insert("test-rule".into(), DiagnosticLevel::Allow);
        assert!(!config.rule_enabled("test-rule"), "allowed rule should be disabled");
    }

    #[test]
    fn load_returns_defaults_when_no_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let config = load(tmp.path());
        assert_eq!(config.max_expects, 5, "should return default max_expects when no Cargo.toml");
    }

    #[test]
    fn load_parses_workspace_lints() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo_toml(
            tmp.path(),
            r#"
[workspace]
members = ["crates/foo"]

[workspace.lints.cargo-test-lint]
max-expects = 10

[workspace.lints.cargo-test-lint.rules]
sleepy-test = "deny"
"#,
        );

        let config = load(tmp.path());
        assert_eq!(config.max_expects, 10, "workspace lints should set max_expects to 10");
        assert_eq!(
            config.rule_level("sleepy-test", DiagnosticLevel::Warn),
            DiagnosticLevel::Deny,
            "workspace lints should override sleepy-test to deny"
        );
    }

    #[test]
    fn load_parses_package_level_lints() {
        let tmp = tempfile::tempdir().unwrap();
        write_cargo_toml(
            tmp.path(),
            r#"
[package]
name = "test-crate"

[lints.cargo-test-lint]
max-nested-mod = 2
"#,
        );

        let config = load(tmp.path());
        assert_eq!(config.max_nested_mod, 2, "package lints should set max_nested_mod to 2");
    }
}
