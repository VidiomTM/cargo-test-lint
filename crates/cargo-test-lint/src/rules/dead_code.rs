use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::collections::HashSet;
use tree_sitter::{Node, QueryMatch};

pub struct DeadTestHelper;

impl Rule for DeadTestHelper {
    fn id(&self) -> &'static str {
        "CTL_DEAD_TEST_HELPER"
    }
    fn config_key(&self) -> &'static str {
        "dead-test-helper"
    }
    fn description(&self) -> &'static str {
        "unused test helper"
    }
    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }
    fn query_str(&self) -> &'static str {
        r#"(mod_item
            name: (identifier) @mod_name
            body: (declaration_list) @body) @mod"#
    }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let name_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let body_node = query_match.captures.iter().find(|c| c.index == 1).map(|c| c.node);
        let mod_node = query_match.captures.iter().find(|c| c.index == 2).map(|c| c.node);
        let (Some(node), Some(name), Some(body)) = (mod_node, name_node, body_node) else {
            return vec![];
        };
        let mod_name = name.utf8_text(ctx.source).unwrap_or("");
        if !is_test_module(&node, ctx.source, mod_name) {
            return vec![];
        }
        let definitions = collect_definitions(&body, ctx.source);
        if definitions.is_empty() {
            return vec![];
        }
        let references = collect_references(&body, ctx.source);
        let mut diagnostics = Vec::new();
        for (def_name, def_node) in &definitions {
            if !references.contains(def_name.as_str()) {
                diagnostics.push(Diagnostic {
                    rule_id: self.id().into(),
                    level: self.default_level(),
                    message: format!(
                        "unused test helper `{def_name}` — defined but never referenced"
                    ),
                    file_path: ctx.file_path.to_path_buf(),
                    line: def_node.start_position().row + 1,
                    column: def_node.start_position().column + 1,
                    end_line: def_node.end_position().row + 1,
                    end_column: def_node.end_position().column + 1,
                    suggestion: None,
                });
            }
        }
        diagnostics
    }
}

fn is_test_module(node: &Node, source: &[u8], name: &str) -> bool {
    if name == "tests" || name == "test" {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = child.utf8_text(source).unwrap_or("");
            if text.contains("cfg(test)") {
                return true;
            }
        }
    }
    false
}

fn collect_definitions<'a>(body: &Node<'a>, source: &[u8]) -> Vec<(String, Node<'a>)> {
    let mut defs = Vec::new();
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        let kind = child.kind();
        let name_field = match kind {
            "function_item" | "struct_item" | "enum_item" | "type_item" | "trait_item" => {
                child.child_by_field_name("name")
            }
            _ => None,
        };
        if let Some(name_node) = name_field {
            if has_test_attr(&child, source) {
                continue;
            }
            let name = name_node.utf8_text(source).unwrap_or("").to_string();
            defs.push((name, child));
        }
    }
    defs
}

fn has_test_attr(node: &Node, source: &[u8]) -> bool {
    // In tree-sitter-rust, #[test] is a sibling attribute_item preceding the function_item
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            let text = child.utf8_text(source).unwrap_or("");
            if text.contains("test") {
                return true;
            }
        }
    }
    // Also check preceding siblings for attribute_item
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "attribute_item" {
            let text = sib.utf8_text(source).unwrap_or("");
            if text.contains("test") {
                return true;
            }
            prev = sib.prev_sibling();
        } else {
            break;
        }
    }
    false
}

fn collect_references(body: &Node, source: &[u8]) -> HashSet<String> {
    let mut refs = HashSet::new();
    let mut all = Vec::new();
    find_descendants(*body, &mut all);
    for node in all {
        if node.kind() == "identifier" || node.kind() == "type_identifier" {
            if is_def_name(&node) {
                continue;
            }
            let text = node.utf8_text(source).unwrap_or("");
            refs.insert(text.to_string());
        }
    }
    refs
}

fn is_def_name(node: &Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.child_by_field_name("name").map(|n| n.id()) == Some(node.id()) {
        matches!(
            parent.kind(),
            "function_item" | "struct_item" | "enum_item" | "type_item" | "trait_item"
        )
    } else {
        false
    }
}

fn find_descendants<'a>(node: Node<'a>, acc: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        acc.push(child);
        find_descendants(child, acc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn unused_function_flagged() {
        let source = r#"#[cfg(test)] mod tests { fn helper() -> u32 { 42 } #[test] fn test_foo() { assert_eq!(1, 1); } }"#;
        let diags = test_rule(&DeadTestHelper, source);
        assert_eq!(diags.len(), 1, "unused function should produce exactly 1 diagnostic");
        assert!(
            diags[0].message.contains("helper"),
            "message should reference unused function name"
        );
    }

    #[test]
    fn used_function_passes() {
        let source = r#"#[cfg(test)] mod tests { fn helper() -> u32 { 42 } #[test] fn test_foo() { assert_eq!(helper(), 42); } }"#;
        assert_eq!(
            test_rule(&DeadTestHelper, source).len(),
            0,
            "used function should produce no diagnostics"
        );
    }

    #[test]
    fn unused_struct_flagged() {
        let source = r#"#[cfg(test)] mod tests { struct TestData { value: u32 } #[test] fn test_foo() { assert_eq!(1, 1); } }"#;
        let diags = test_rule(&DeadTestHelper, source);
        assert_eq!(diags.len(), 1, "unused struct should produce exactly 1 diagnostic");
        assert!(
            diags[0].message.contains("TestData"),
            "message should reference unused struct name"
        );
    }

    #[test]
    fn non_test_module_ignored() {
        let source = r#"mod helpers { fn unused() {} }"#;
        assert_eq!(
            test_rule(&DeadTestHelper, source).len(),
            0,
            "non-test module should be ignored"
        );
    }

    #[test]
    fn multiple_unused_flagged() {
        let source = r#"#[cfg(test)] mod tests { fn helper_a() {} fn helper_b() {} #[test] fn test_foo() { assert_eq!(1, 1); } }"#;
        assert_eq!(
            test_rule(&DeadTestHelper, source).len(),
            2,
            "two unused helpers should produce 2 diagnostics"
        );
    }
}
