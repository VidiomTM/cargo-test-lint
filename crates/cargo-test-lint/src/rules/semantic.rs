use super::test_context::is_in_test_function;
use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StringLiteralCorpus;

const EMBEDDED_TEST_SIGNALS: &[&str] = &[
    "def test_",
    "it('",
    "it(\"",
    "describe('",
    "describe(\"",
    "expect(",
    "import pytest",
    "from pytest",
    "from vitest",
    "import vitest",
];

impl Rule for StringLiteralCorpus {
    fn id(&self) -> &'static str {
        "CTL_STRING_CORPUS"
    }
    fn config_key(&self) -> &'static str {
        "string-literal-corpus"
    }
    fn description(&self) -> &'static str {
        "test corpus code embedded in string literal"
    }
    fn default_level(&self) -> DiagnosticLevel {
        DiagnosticLevel::Warn
    }
    fn query_str(&self) -> &'static str {
        r#"[
            (string_literal) @str
            (raw_string_literal) @str
        ]"#
    }
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic> {
        let str_node = query_match.captures.iter().find(|c| c.index == 0).map(|c| c.node);
        let Some(node) = str_node else {
            return vec![];
        };
        let text = node.utf8_text(ctx.source).unwrap_or("");
        if text.len() < 40 {
            return vec![];
        }

        let is_embedded = EMBEDDED_TEST_SIGNALS.iter().any(|sig| text.contains(sig));
        if !is_embedded {
            return vec![];
        }

        if !is_in_test_function(node, ctx.source) {
            return vec![];
        }

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "test corpus code embedded in string literal ({} chars) — extract to fixture file",
                text.len(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn embedded_python_test_code_flagged() {
        let source = include_str!("../../tests/fixtures/semantic_python_corpus.rs");
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(!diags.is_empty(), "expected diagnostics for embedded python test code");
    }

    #[test]
    fn short_string_ignored() {
        let source = r##"
#[cfg(test)]
mod tests {
    #[test]
    fn test_short() {
        let name = "hello world";
        assert_eq!(name, "hello world");
    }
}
"##;
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(diags.is_empty(), "expected no diagnostics for short string");
    }

    #[test]
    fn no_test_context_ignored() {
        let source = include_str!("../../tests/fixtures/semantic_no_test_context.rs");
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(diags.is_empty(), "expected no diagnostics outside test context");
    }

    #[test]
    fn embedded_jest_test_code_flagged() {
        let source = include_str!("../../tests/fixtures/semantic_jest_corpus.rs");
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(!diags.is_empty(), "expected diagnostics for embedded jest test code");
    }
}
