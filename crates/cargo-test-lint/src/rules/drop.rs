use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct MissingDropGuard;

const RESOURCE_ALLOCATORS: &[&str] = &[
    "File::create",
    "File::open",
    "TempDir::new",
    "tempfile::tempdir",
    "tempdir",
    "tempfile::NamedTempFile::new",
    "NamedTempFile::new",
    "Builder::new",
    "TcpListener::bind",
    "UdpSocket::bind",
];

impl Rule for MissingDropGuard {
    fn id(&self) -> &'static str {
        "CTL_MISSING_DROP_GUARD"
    }
    fn config_key(&self) -> &'static str {
        "missing-drop-guard"
    }
    fn description(&self) -> &'static str {
        "resource allocation without RAII guard"
    }
    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }
    fn query_str(&self) -> &'static str {
        r#"(call_expression) @call"#
    }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let Some(node) = call_node else {
            return vec![];
        };
        let func_node = node.child_by_field_name("function");
        let Some(func) = func_node else {
            return vec![];
        };
        let func_text = func.utf8_text(ctx.source).unwrap_or("");
        let is_resource = RESOURCE_ALLOCATORS
            .iter()
            .any(|alloc| func_text == *alloc || func_text.ends_with(&format!("::{alloc}")));
        if !is_resource {
            return vec![];
        }

        // Traverse ancestors to find if this call is ultimately bound by a let declaration.
        // This handles common patterns like:
        //   let dir = TempDir::new().unwrap();       // bound
        //   let dir = TempDir::new()?;                // bound
        //   write_file(TempDir::new().unwrap())       // not bound
        //   TempDir::new();                           // not bound
        if ancestor_is_let_declaration(node) {
            return vec![];
        }

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "resource allocation `{func_text}` without RAII guard — may leak on assertion panic"
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

/// Walk up ancestor nodes to find if this expression is ultimately
/// bound by a `let` declaration. Handles:
///   - Direct binding: `let x = Foo::new()`
///   - Through .unwrap(): `let x = Foo::new().unwrap()`
///   - Through ?: `let x = Foo::new()?`
///   - Through method chains: `let x = Foo::new().method().unwrap()`
fn ancestor_is_let_declaration(node: tree_sitter::Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "let_declaration" => {
                let is_wildcard = parent
                    .child_by_field_name("pattern")
                    .map(|p| p.kind() == "_" || p.kind() == "wildcard_pattern")
                    .unwrap_or(false);
                if is_wildcard {
                    return false;
                }
                return true;
            }
            // If we hit a statement that is NOT a let, the call chain
            // is not bound. But only stop at direct children of
            // expression_statement or declaration_list levels,
            // not at intermediate method call nodes.
            "expression_statement" => return false,
            "closure" => return false,
            "ERROR" => return false,
            _ => {}
        }
        current = parent.parent();
    }
    false
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
    fn bound_with_unwrap_passes() {
        let source = r#"#[test] fn test_foo() { let dir = TempDir::new().unwrap(); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn bound_with_question_mark_passes() {
        let source =
            r#"#[test] fn test_foo() -> Result<(), Error> { let dir = TempDir::new()?; Ok(()) }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn bound_with_method_chain_passes() {
        let source = r#"#[test] fn test_foo() { let file = tempfile::NamedTempFile::new().unwrap().path().to_path_buf(); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn unbound_tempdir_flagged() {
        let source = r#"#[test] fn test_foo() { TempDir::new(); }"#;
        let diags = test_rule(&MissingDropGuard, source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn tempfile_tempdir_recognized() {
        let source = r#"#[test] fn test_foo() { let dir = tempfile::tempdir(); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }

    #[test]
    fn unbound_tempfile_tempdir_flagged() {
        let source = r#"#[test] fn test_foo() { tempfile::tempdir(); }"#;
        let diags = test_rule(&MissingDropGuard, source);
        assert_eq!(diags.len(), 1);
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

    #[test]
    fn wildcard_let_flagged() {
        let source = r#"#[test] fn test_foo() { let _ = TempDir::new().unwrap(); }"#;
        let diags = test_rule(&MissingDropGuard, source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn passed_as_argument_not_let_bound_still_flagged() {
        // Even though the function call itself is an argument,
        // the value is not stored in a let binding that lives
        // for the duration of the test. It's effectively unbound.
        let source = r#"#[test] fn test_foo() { write_file(File::create("test.txt")); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 1);
    }

    #[test]
    fn let_bound_then_passed_ok() {
        let source =
            r#"#[test] fn test_foo() { let f = File::create("test.txt"); write_file(f); }"#;
        assert_eq!(test_rule(&MissingDropGuard, source).len(), 0);
    }
}
