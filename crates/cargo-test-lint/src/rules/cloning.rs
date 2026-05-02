use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::{Node, QueryMatch};

pub struct UnnecessaryClone;

fn collect_descendants<'a>(node: Node<'a>, acc: &mut Vec<Node<'a>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        acc.push(child);
        collect_descendants(child, acc);
    }
}

impl Rule for UnnecessaryClone {
    fn id(&self) -> &'static str { "CTL_UNNECESSARY_CLONE" }
    fn config_key(&self) -> &'static str { "unnecessary-clone" }
    fn description(&self) -> &'static str { "unnecessary .clone()" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { r#"(call_expression) @call"# }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let Some(node) = call_node else { return vec![]; };
        let func_node = node.child_by_field_name("function");
        let Some(func) = func_node else { return vec![]; };
        if func.kind() != "field_expression" { return vec![]; }
        let field_child = func.child_by_field_name("field");
        let Some(field) = field_child else { return vec![]; };
        if field.utf8_text(ctx.source).unwrap_or("") != "clone" { return vec![]; }
        let object = func.child_by_field_name("value");
        let Some(obj) = object else { return vec![]; };
        if obj.kind() != "identifier" { return vec![]; }
        let obj_text = obj.utf8_text(ctx.source).unwrap_or("");
        let parent = node.parent();
        let is_let_rhs = parent.map(|p| p.kind() == "let_declaration").unwrap_or(false);
        if !is_let_rhs { return vec![]; }
        let let_parent = parent.unwrap();
        let scope_parent = let_parent.parent();
        let Some(scope) = scope_parent else { return vec![]; };
        let clone_end = node.end_byte();
        let mut descendants = Vec::new();
        collect_descendants(scope, &mut descendants);
        let found_usage = descendants.iter().any(|d| {
            d.byte_range().start >= clone_end
                && d.kind() == "identifier"
                && d.utf8_text(ctx.source).unwrap_or("") == obj_text
        });
        if !found_usage {
            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!("value `{obj_text}` cloned but not reused — consider borrowing instead"),
                file_path: ctx.file_path.to_path_buf(),
                line: node.start_position().row + 1,
                column: node.start_position().column + 1,
                end_line: node.end_position().row + 1,
                end_column: node.end_position().column + 1,
                suggestion: None,
            }]
        } else { vec![] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn clone_not_reused_flagged() {
        let source = r#"#[test] fn test_foo() { let x = vec![1, 2, 3]; let y = x.clone(); assert_eq!(y, vec![1, 2, 3]); }"#;
        let diags = test_rule(&UnnecessaryClone, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("x"));
    }

    #[test]
    fn clone_reused_passes() {
        let source = r#"#[test] fn test_foo() { let x = vec![1, 2, 3]; let y = x.clone(); assert_eq!(y, vec![1, 2, 3]); assert_eq!(x.len(), 3); }"#;
        let diags = test_rule(&UnnecessaryClone, source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn non_let_clone_ignored() {
        let source = r#"#[test] fn test_foo() { let x = vec![1, 2, 3]; foo(x.clone()); assert_eq!(x.len(), 3); }"#;
        let diags = test_rule(&UnnecessaryClone, source);
        assert_eq!(diags.len(), 0);
    }
}
