use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::diagnostics::DiagnosticLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
    #[serde(default)]
    pub max_expects: usize,
    #[serde(default)]
    pub max_nested_mod: usize,
    #[serde(default)]
    pub nextest: bool,
    #[serde(default)]
    pub deny_warnings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleConfig {
    Level(DiagnosticLevel),
    Threshold(usize),
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert("assertion-roulette".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("sleepy-test".into(), RuleConfig::Level(DiagnosticLevel::Forbid));
        rules.insert("test-branching".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("async-blocking".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("unnecessary-clone".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("deep-wrapper".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("missing-drop-guard".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("dead-test-helper".into(), RuleConfig::Level(DiagnosticLevel::Warn));
        rules.insert("nextest-compatibility".into(), RuleConfig::Level(DiagnosticLevel::Warn));

        Self {
            rules,
            max_expects: 5,
            max_nested_mod: 3,
            nextest: false,
            deny_warnings: false,
        }
    }
}

impl Config {
    pub fn rule_level(&self, rule_id: &str, default: DiagnosticLevel) -> DiagnosticLevel {
        match self.rules.get(rule_id) {
            Some(RuleConfig::Level(level)) => level.clone(),
            _ => default,
        }
    }

    pub fn rule_enabled(&self, rule_id: &str) -> bool {
        !matches!(
            self.rules.get(rule_id),
            Some(RuleConfig::Level(DiagnosticLevel::Allow))
        )
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

    // Try workspace-level lints first, then package-level
    if let Ok(manifest) = toml::from_str::<Manifest>(&content) {
        if let Some(ws) = manifest.workspace {
            if let Some(lints) = ws.lints {
                if let Some(config) = lints.cargo_test_lint {
                    return config;
                }
            }
        }
        if let Some(lints) = manifest.lints {
            if let Some(config) = lints.cargo_test_lint {
                return config;
            }
        }
    }

    Config::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn default_config_has_all_rules() {
        let config = Config::default();
        assert!(config.rule_enabled("assertion-roulette"));
        assert!(config.rule_enabled("sleepy-test"));
        assert!(config.rule_enabled("test-branching"));
        assert_eq!(config.max_expects, 5);
        assert_eq!(config.max_nested_mod, 3);
        assert!(!config.nextest);
    }

    #[test]
    fn rule_level_returns_default_when_not_configured() {
        let config = Config::default();
        assert_eq!(
            config.rule_level("nonexistent", DiagnosticLevel::Deny),
            DiagnosticLevel::Deny
        );
    }

    #[test]
    fn rule_enabled_false_when_allowed() {
        let mut config = Config::default();
        config.rules.insert("test-rule".into(), RuleConfig::Level(DiagnosticLevel::Allow));
        assert!(!config.rule_enabled("test-rule"));
    }

    #[test]
    fn load_returns_defaults_when_no_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let config = load(tmp.path());
        assert_eq!(config.max_expects, 5);
    }

    #[test]
    fn load_parses_workspace_lints() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/foo"]

[workspace.lints.cargo-test-lint]
max-expects = 10

[workspace.lints.cargo-test-lint.rules]
sleepy-test = "deny"
"#,
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.max_expects, 10);
        assert_eq!(
            config.rule_level("sleepy-test", DiagnosticLevel::Warn),
            DiagnosticLevel::Deny
        );
    }

    #[test]
    fn load_parses_package_level_lints() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"
[package]
name = "test-crate"

[lints.cargo-test-lint]
max-nested-mod = 2
"#,
        )
        .unwrap();

        let config = load(tmp.path());
        assert_eq!(config.max_nested_mod, 2);
    }
}
