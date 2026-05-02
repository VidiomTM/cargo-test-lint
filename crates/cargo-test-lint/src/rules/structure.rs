use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct NestedMod;

impl Rule for NestedMod {
    fn id(&self) -> &'static str {
        "CTL_NESTED_MOD"
    }
    fn config_key(&self) -> &'static str {
        "nested-mod"
    }
    fn description(&self) -> &'static str {
        "deeply nested test module"
    }
    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }
    fn query_str(&self) -> &'static str {
        r#"(mod_item
            name: (identifier) @name) @mod"#
    }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let name_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let mod_node = query_match.captures.iter().find(|c| c.index == 1).map(|c| c.node);
        let (Some(node), Some(name)) = (mod_node, name_node) else {
            return vec![];
        };
        let name_str = name.utf8_text(ctx.source).unwrap_or("");
        if !is_test_module(&node, ctx.source, name_str) {
            return vec![];
        }
        let depth = mod_nesting_depth(&node);
        let threshold = ctx.config.max_nested_mod;
        if threshold == 0 || depth <= threshold {
            return vec![];
        }
        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "test module nesting depth {depth} (max {threshold}) — flatten structure"
            ),
            file_path: ctx.file_path.to_path_buf(),
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

fn is_test_module(node: &tree_sitter::Node, source: &[u8], name: &str) -> bool {
    if name == "tests" || name == "test" {
        return true;
    }
    if let Some(parent) = node.parent() {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            if child == *node {
                break;
            }
            if child.kind() == "attribute_item" {
                let text = child.utf8_text(source).unwrap_or("");
                if text.contains("cfg(test)") {
                    return true;
                }
            }
        }
    }
    false
}

fn mod_nesting_depth(node: &tree_sitter::Node) -> usize {
    let mut depth = 1;
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "mod_item" {
            depth += 1;
        }
        current = parent.parent();
    }
    depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::rules::test_rule_with_config;

    fn config_with_max(max: usize) -> Config {
        Config { max_nested_mod: max, ..Default::default() }
    }

    #[test]
    fn shallow_test_mod_passes() {
        let source = r#"#[cfg(test)] mod tests { #[test] fn test_foo() {} }"#;
        let diags = test_rule_with_config(&NestedMod, source, config_with_max(3));
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn deeply_nested_test_mod_flagged() {
        let source = r#"mod outer { mod inner { mod tests { #[test] fn test_foo() {} } } }"#;
        let diags = test_rule_with_config(&NestedMod, source, config_with_max(2));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("depth 3"));
    }

    #[test]
    fn non_test_mod_ignored() {
        let source = r#"mod a { mod b { mod c { fn helper() {} } } }"#;
        let diags = test_rule_with_config(&NestedMod, source, config_with_max(2));
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn cfg_test_attribute_detected() {
        let source =
            r#"mod a { mod b { #[cfg(test)] mod my_tests { #[test] fn test_foo() {} } } }"#;
        let diags = test_rule_with_config(&NestedMod, source, config_with_max(2));
        assert_eq!(diags.len(), 1);
    }
}
