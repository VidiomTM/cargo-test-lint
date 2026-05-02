use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct MissingDropGuard;

const RESOURCE_ALLOCATORS: &[&str] = &[
    "File::create", "File::open", "TempDir::new", "Builder::new",
    "TcpListener::bind", "UdpSocket::bind",
];

impl Rule for MissingDropGuard {
    fn id(&self) -> &'static str { "CTL_MISSING_DROP_GUARD" }
    fn config_key(&self) -> &'static str { "missing-drop-guard" }
    fn description(&self) -> &'static str { "resource allocation without RAII guard" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { r#"(call_expression) @call"# }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let Some(node) = call_node else { return vec![]; };
        let func_node = node.child_by_field_name("function");
        let Some(func) = func_node else { return vec![]; };
        let func_text = func.utf8_text(ctx.source).unwrap_or("");
        let is_resource = RESOURCE_ALLOCATORS.iter().any(|alloc| func_text.contains(alloc));
        if !is_resource { return vec![]; }
        let parent = node.parent();
        let is_bound = parent.map(|p| p.kind() == "let_declaration").unwrap_or(false);
        if is_bound { return vec![]; }
        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!("resource allocation `{func_text}` without RAII guard — may leak on assertion panic"),
            file_path: ctx.file_path.to_path_buf(),
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn unbound_file_create_flagged() {
        let source = r#"#[test] fn test_foo() { File::create("test.txt"); }"#;
        let diags = test_rule(&MissingDropGuard, source);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("File::create"));
    }

    #[test]
    fn bound_file_create_passes() {
        let source = r#"#[test] fn test_foo() { let f = File::create("test.txt"); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn non_resource_call_ignored() {
        let source = r#"#[test] fn test_foo() { println!("hello"); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn tcp_listener_without_binding_flagged() {
        let source = r#"#[test] fn test_foo() { TcpListener::bind("127.0.0.1:0"); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 1);
    }
}
