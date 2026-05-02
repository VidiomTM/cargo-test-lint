use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct SleepyTest;

impl Rule for SleepyTest {
    fn id(&self) -> &'static str {
        "CTL_SLEEP"
    }

    fn config_key(&self) -> &'static str {
        "sleepy-test"
    }

    fn description(&self) -> &'static str {
        "thread::sleep in test code"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Forbid
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression
            function: (scoped_identifier
                path: (scoped_identifier
                    path: (identifier) @path
                    name: (identifier) @mid
                    (#eq? @path "std")
                    (#eq? @mid "thread"))
                name: (identifier) @name
                (#eq? @name "sleep"))) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let call_node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = call_node else {
            return vec![];
        };

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: "thread::sleep in test — use tokio::time::advance() or mock clock".into(),
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

    fn rule() -> SleepyTest {
        SleepyTest
    }

    #[test]
    fn std_sleep_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_SLEEP");
    }

    #[test]
    fn non_sleep_call_ignored() {
        let source = r#"
#[test]
fn test_foo() {
    println!("hello");
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn other_sleep_not_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    my_module::sleep(100);
}
"#;
        let diags = test_rule(&rule(), source);
        assert_eq!(diags.len(), 0);
    }
}
