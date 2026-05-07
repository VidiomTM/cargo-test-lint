use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct TestBranching;

impl Rule for TestBranching {
    fn id(&self) -> &'static str {
        "CTL_TEST_BRANCHING"
    }

    fn config_key(&self) -> &'static str {
        "test-branching"
    }

    fn description(&self) -> &'static str {
        "control flow in test body"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(function_item
            name: (identifier) @fn_name
            body: (block) @body) @fn"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let fn_capture = query_match.captures.iter().find(|c| c.index == 0);
        let body_capture = query_match.captures.iter().find(|c| c.index == 1);

        let (Some(fn_node), Some(body_node)) =
            (fn_capture.map(|c| c.node), body_capture.map(|c| c.node))
        else {
            return vec![];
        };

        let fn_name = fn_node.utf8_text(ctx.source).unwrap_or("");
        if !fn_name.starts_with("test") {
            return vec![];
        }

        let mut diagnostics = Vec::new();
        collect_branching(body_node, ctx, self, &mut diagnostics);
        diagnostics
    }
}

fn collect_branching(
    node: tree_sitter::Node,
    ctx: &RuleContext,
    rule: &TestBranching,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "if_expression" | "match_expression" | "if_let_expression" => {
                diagnostics.push(Diagnostic {
                    rule_id: rule.id().into(),
                    level: rule.default_level(),
                    message: format!(
                        "test contains {} — tests should be deterministic",
                        child.kind()
                    ),
                    file_path: ctx.file_path.to_path_buf(),
                    line: child.start_position().row + 1,
                    column: child.start_position().column + 1,
                    end_line: child.end_position().row + 1,
                    end_column: child.end_position().column + 1,
                    suggestion: None,
                });
            }
            _ => {
                collect_branching(child, ctx, rule, diagnostics);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> TestBranching {
        TestBranching
    }

    #[test]
    fn if_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    if true {
        assert!(true);
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1, "if in test should produce exactly 1 diagnostic");
        assert_eq!(
            diags[0].rule_id, "CTL_TEST_BRANCHING",
            "diagnostic should be CTL_TEST_BRANCHING"
        );
    }

    #[test]
    fn match_in_test_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    match 42 {
        _ => assert!(true),
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1, "match in test should produce exactly 1 diagnostic");
    }

    #[test]
    fn no_branching_passes() {
        let source = r#"
#[test]
fn test_foo() {
    let x = 42;
    assert_eq!(x, 42);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0, "test without branching should produce no diagnostics");
    }

    #[test]
    fn non_test_function_ignored() {
        let source = r#"
fn helper() {
    if true {
        println!("ok");
    }
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0, "non-test functions should not be flagged");
    }
}
