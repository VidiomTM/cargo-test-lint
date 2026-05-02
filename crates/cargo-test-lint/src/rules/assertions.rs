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
        let token_tree = macro_node.named_children(&mut cursor).find(|c| c.kind() == "token_tree");
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
    fn id(&self) -> &'static str {
        "CTL_MAX_EXPECTS"
    }

    fn config_key(&self) -> &'static str {
        "max-expects"
    }

    fn description(&self) -> &'static str {
        "too many assertions in test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"((attribute_item
            (attribute
                (identifier) @attr_name
                (#eq? @attr_name "test")))
            .
            (function_item
                name: (identifier) @fn_name
                body: (block) @body) @fn)"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let body_capture = query_match.captures.iter().find(|c| c.index == 2);
        let fn_capture = query_match.captures.iter().find(|c| c.index == 3);

        let (Some(body_node), Some(fn_node)) =
            (body_capture.map(|c| c.node), fn_capture.map(|c| c.node))
        else {
            return vec![];
        };

        let threshold = ctx.config.max_expects;
        if threshold == 0 {
            return vec![];
        }

        let assert_count = count_assertions(&body_node, ctx.source);

        if assert_count > threshold {
            let line = fn_node.start_position().row + 1;
            let col = fn_node.start_position().column + 1;

            vec![Diagnostic {
                rule_id: self.id().into(),
                level: self.default_level(),
                message: format!(
                    "test has {assert_count} assertions (max {threshold}) — consider splitting"
                ),
                file_path: ctx.file_path.to_path_buf(),
                line,
                column: col,
                end_line: fn_node.end_position().row + 1,
                end_column: fn_node.end_position().column + 1,
                suggestion: None,
            }]
        } else {
            vec![]
        }
    }
}

fn count_assertions(body: &tree_sitter::Node, source: &[u8]) -> usize {
    let mut count = 0;
    count_assertions_inner(body, source, &mut count);
    count
}

fn count_assertions_inner(node: &tree_sitter::Node, source: &[u8], count: &mut usize) {
    if node.kind() == "macro_invocation" {
        if let Some(name_node) = node.child_by_field_name("macro") {
            let name = name_node.utf8_text(source).unwrap_or("");
            if name.starts_with("assert") {
                *count += 1;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        count_assertions_inner(&child, source, count);
    }
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

    // MaxExpects tests
    use crate::config::Config;
    use crate::rules::test_rule_with_config;

    fn config_with_max(max: usize) -> Config {
        let mut config = Config::default();
        config.max_expects = max;
        config
    }

    #[test]
    fn under_threshold_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert_eq!(1, 1);
}
"#;
        let config = config_with_max(5);
        let diags = test_rule_with_config(&MaxExpects, source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn over_threshold_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
}
"#;
        let config = config_with_max(5);
        let diags = test_rule_with_config(&MaxExpects, source, config);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("6 assertions"));
        assert!(diags[0].message.contains("max 5"));
    }

    #[test]
    fn at_threshold_passes() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
    assert!(true);
    assert!(true);
}
"#;
        let config = config_with_max(3);
        let diags = test_rule_with_config(&MaxExpects, source, config);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn zero_threshold_disables_check() {
        let source = r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#;
        let config = config_with_max(0);
        let diags = test_rule_with_config(&MaxExpects, source, config);
        assert_eq!(diags.len(), 0);
    }
}
