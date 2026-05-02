use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel, Fix};
use tree_sitter::QueryMatch;

pub struct AssertMsg;
pub struct MaxExpects;

impl Rule for AssertMsg {
    fn id(&self) -> &'static str {
        "CTL_ASSERT_MSG"
    }

    fn config_key(&self) -> &'static str {
        "assertion-roulette"
    }

    fn description(&self) -> &'static str {
        "assertion missing context message"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(macro_invocation
            macro: (identifier) @name
            (#match? @name "^assert(_eq|_ne)?$")) @macro"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        // Capture order in query: @name is index 0, @macro is index 1
        let name_capture = query_match.captures.iter().find(|c| c.index == 0);
        let macro_capture = query_match.captures.iter().find(|c| c.index == 1);

        let (Some(name_cap), Some(macro_cap)) = (name_capture, macro_capture) else {
            return vec![];
        };

        let macro_node = macro_cap.node;
        let name = &ctx.source[name_cap.node.byte_range()];

        // Find token_tree as a named child (not a field)
        let mut cursor = macro_node.walk();
        let token_tree = macro_node.named_children(&mut cursor)
            .find(|c| c.kind() == "token_tree");
        let Some(tt) = token_tree else {
            return vec![];
        };

        // Count comma-separated arguments
        let arg_count = count_macro_args(&tt);

        // assert! needs 2+ args (condition + message)
        // assert_eq!/assert_ne! needs 3+ args (left, right + message)
        let min_args = if name == b"assert" { 2 } else { 3 };

        if arg_count < min_args {
            let node_range = macro_node.byte_range();
            let line = macro_node.start_position().row + 1;
            let col = macro_node.start_position().column + 1;
            let end_line = macro_node.end_position().row + 1;
            let end_col = macro_node.end_position().column + 1;

            let suggestion = build_suggestion(name, &tt, ctx.source);

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: "assertion missing context message — add a format string for readable CI failures".into(),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line,
                end_column: end_col,
                suggestion: suggestion.map(|s| Fix {
                    description: "add context message".into(),
                    replacement: s,
                    start_byte: node_range.start,
                    end_byte: node_range.end,
                }),
            }]
        } else {
            vec![]
        }
    }
}

fn count_macro_args(token_tree: &tree_sitter::Node) -> usize {
    let mut cursor = token_tree.walk();
    let mut comma_count = 0;
    let mut has_content = false;
    for child in token_tree.children(&mut cursor) {
        if child.is_named() {
            has_content = true;
        }
        if child.kind() == "," {
            comma_count += 1;
        }
    }
    if has_content { comma_count + 1 } else { 0 }
}

fn build_suggestion(name: &[u8], token_tree: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    let original = &source[token_tree.byte_range()];
    let original_str = std::str::from_utf8(original).ok()?;
    let macro_name = std::str::from_utf8(name).ok()?;

    // Strip outer parens
    let inner = original_str.strip_prefix('(')?.strip_suffix(')')?;

    Some(format!("{macro_name}!({inner}, \"TODO: add context message\")"))
}

impl Rule for MaxExpects {
    fn id(&self) -> &'static str { "CTL_MAX_EXPECTS" }
    fn config_key(&self) -> &'static str { "max-expects" }
    fn description(&self) -> &'static str { "too many assertions in test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> AssertMsg {
        AssertMsg
    }

    #[test]
    fn assert_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_ASSERT_MSG");
    }

    #[test]
    fn assert_with_message_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true, "should be true");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn assert_eq_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1 + 1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn assert_eq_with_message_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1 + 1, 2, "math should work");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn assert_ne_without_message_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert_ne!(1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn non_assert_macros_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    println!("hello");
    vec![1, 2, 3];
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn suggestion_includes_message_placeholder() {
        let source = r#"
#[test]
fn test_foo() {
    assert_eq!(1, 2);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        let fix = diags[0].suggestion.as_ref().unwrap();
        assert!(fix.replacement.contains("TODO: add context message"));
    }
}
