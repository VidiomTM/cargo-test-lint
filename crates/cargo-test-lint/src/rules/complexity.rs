use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeepWrapper;

impl Rule for DeepWrapper {
    fn id(&self) -> &'static str {
        "CTL_DEEP_WRAPPER"
    }
    fn config_key(&self) -> &'static str {
        "deep-wrapper"
    }
    fn description(&self) -> &'static str {
        "deeply nested type wrapper"
    }
    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }
    fn query_str(&self) -> &'static str {
        r#"(type_item
            name: (type_identifier) @name
            type: (_) @ty) @type_item"#
    }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let ty_capture = query_match.captures.iter().find(|c| c.index == 2);
        let item_capture = query_match.captures.iter().find(|c| c.index == 0);
        let (Some(ty_node), Some(item_node)) =
            (ty_capture.map(|c| c.node), item_capture.map(|c| c.node))
        else {
            return vec![];
        };
        let depth = count_generic_depth(&ty_node, 0);
        if depth > 3 {
            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!(
                    "deeply nested type wrapper ({depth} levels) — test setup is overly complex"
                ),
                file_path: ctx.file_path.to_path_buf(),
                line: item_node.start_position().row + 1,
                column: item_node.start_position().column + 1,
                end_line: item_node.end_position().row + 1,
                end_column: item_node.end_position().column + 1,
                suggestion: None,
            }]
        } else {
            vec![]
        }
    }
}

fn count_generic_depth(node: &tree_sitter::Node, current: usize) -> usize {
    let kind = node.kind();
    let is_wrapper =
        matches!(kind, "generic_type" | "tuple_type" | "reference_type" | "array_type");
    let next = if is_wrapper { current + 1 } else { current };
    let mut max_depth = next;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let child_depth = count_generic_depth(&child, if is_wrapper { next } else { current });
        max_depth = max_depth.max(child_depth);
    }
    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn simple_type_passes() {
        let source = r#"type MyType = Vec<u32>;"#;
        assert_eq!(
            test_rule(&DeepWrapper, source).len(),
            0,
            "simple type should produce no diagnostics"
        );
    }

    #[test]
    fn deeply_nested_type_flagged() {
        let source = r#"type MyType = Arc<Mutex<Option<HashMap<String, Vec<u32>>>>>;"#;
        let diags = test_rule(&DeepWrapper, source);
        assert_eq!(diags.len(), 1, "deeply nested type should produce exactly 1 diagnostic");
        assert!(diags[0].message.contains("levels"), "message should mention nesting levels");
    }

    #[test]
    fn moderate_nesting_passes() {
        let source = r#"type MyType = Arc<Mutex<u32>>;"#;
        assert_eq!(
            test_rule(&DeepWrapper, source).len(),
            0,
            "moderate nesting should produce no diagnostics"
        );
    }
}
