use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AsyncBlocking;

const BLOCKING_FNS: &[&str] = [
    "std::thread::sleep",
    "std::fs::read",
    "std::fs::write",
    "std::fs::read_to_string",
    "std::net::TcpStream::connect",
    "reqwest::blocking",
    "tokio::runtime::Runtime::new",
]
.as_slice();

impl Rule for AsyncBlocking {
    fn id(&self) -> &'static str {
        "CTL_ASYNC_BLOCKING"
    }

    fn config_key(&self) -> &'static str {
        "async-blocking"
    }

    fn description(&self) -> &'static str {
        "blocking call in async test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(source_file
            (attribute_item
                (attribute
                    (scoped_identifier) @tokio_attr
                    (#eq? @tokio_attr "tokio::test")))
            (function_item) @fn)"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let fn_capture = query_match.captures.iter().find(|c| c.index == 1);
        let Some(fn_node) = fn_capture.map(|c| c.node) else {
            return vec![];
        };

        let mut body_node = None;
        let mut cursor = fn_node.walk();
        for child in fn_node.children(&mut cursor) {
            if child.kind() == "block" {
                body_node = Some(child);
                break;
            }
        }

        let Some(body_node) = body_node else {
            return vec![];
        };

        let mut diagnostics = Vec::new();
        find_blocking_calls(ctx, self, body_node, &mut diagnostics);
        diagnostics
    }
}

fn find_blocking_calls(
    ctx: &RuleContext,
    rule: &AsyncBlocking,
    node: tree_sitter::Node,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            if let Some(func) = child.child_by_field_name("function") {
                let text = func.utf8_text(ctx.source).unwrap_or("");
                for blocking_fn in BLOCKING_FNS {
                    if text.contains(blocking_fn) {
                        diagnostics.push(Diagnostic {
                            rule_id: rule.id().into(),
                            level: rule.default_level(),
                            message: format!(
                                "blocking call `{text}` in async test — use async equivalent"
                            ),
                            file_path: ctx.file_path.to_path_buf(),
                            line: child.start_position().row + 1,
                            column: child.start_position().column + 1,
                            end_line: child.end_position().row + 1,
                            end_column: child.end_position().column + 1,
                            suggestion: None,
                        });
                        break;
                    }
                }
            }
        }
        find_blocking_calls(ctx, rule, child, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    fn rule() -> AsyncBlocking {
        AsyncBlocking
    }

    #[test]
    fn sleep_in_tokio_test_flagged() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(
            diags.len(),
            1,
            "blocking sleep in tokio test should produce exactly 1 diagnostic"
        );
        assert_eq!(
            diags[0].rule_id, "CTL_ASYNC_BLOCKING",
            "diagnostic should be CTL_ASYNC_BLOCKING"
        );
    }

    #[test]
    fn async_code_passes() {
        let source = r#"
#[tokio::test]
async fn test_foo() {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0, "async code should produce no diagnostics");
    }

    #[test]
    fn non_async_test_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0, "non-async tests should not be flagged");
    }
}
