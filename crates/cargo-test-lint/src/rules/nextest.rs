use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StaticMut;
pub struct EnvSetVar;

impl Rule for StaticMut {
    fn id(&self) -> &'static str {
        "CTL_STATIC_MUT"
    }

    fn config_key(&self) -> &'static str {
        "static-mut"
    }

    fn description(&self) -> &'static str {
        "static mutable variable"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(static_item
            (mutable_specifier)) @static"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = node else {
            return vec![];
        };

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: "static mutable variable — incompatible with nextest parallel execution".into(),
            file_path: ctx.file_path.to_path_buf(),
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
            end_line: node.end_position().row + 1,
            end_column: node.end_position().column + 1,
            suggestion: None,
        }]
    }
}

impl Rule for EnvSetVar {
    fn id(&self) -> &'static str {
        "CTL_ENV_SET_VAR"
    }

    fn config_key(&self) -> &'static str {
        "env-set-var"
    }

    fn description(&self) -> &'static str {
        "std::env::set_var in test"
    }

    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }

    fn query_str(&self) -> &'static str {
        r#"(call_expression
            function: (scoped_identifier
                path: (scoped_identifier
                    path: (identifier)
                    name: (identifier) @env)
                name: (identifier) @name
                (#eq? @env "env")
                (#eq? @name "set_var"))) @call"#
    }

    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let node = query_match
            .captures
            .iter()
            .find(|c| c.index == 0)
            .map(|c| c.node);

        let Some(node) = node else {
            return vec![];
        };

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: "std::env::set_var in test — unsafe with nextest parallel execution".into(),
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
    fn static_mut_flagged() {
        let source = r#"
static mut COUNTER: u32 = 0;

#[test]
fn test_foo() {
    unsafe { COUNTER += 1; }
}
"#;
        let diags = test_rule(&StaticMut, source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_STATIC_MUT");
    }

    #[test]
    fn static_const_not_flagged() {
        let source = r#"
const VALUE: u32 = 42;

#[test]
fn test_foo() {
    assert_eq!(VALUE, 42);
}
"#;
        let diags = test_rule(&StaticMut, source);
        assert_eq!(diags.len(), 0);
    }

    #[test]
    fn env_set_var_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    std::env::set_var("MY_VAR", "value");
}
"#;
        let diags = test_rule(&EnvSetVar, source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule_id, "CTL_ENV_SET_VAR");
    }

    #[test]
    fn env_var_not_flagged() {
        let source = r#"
#[test]
fn test_foo() {
    let _ = std::env::var("HOME");
}
"#;
        let diags = test_rule(&EnvSetVar, source);
        assert_eq!(diags.len(), 0);
    }
}
