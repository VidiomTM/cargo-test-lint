use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StringLiteralCorpus;

const EMBEDDED_TEST_SIGNALS: &[&str] = &[
    "def test_",
    "it(",
    "describe(",
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

        if !is_in_test_context(node, ctx.source) {
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

fn is_in_test_context(node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "function_item" | "function_signature"
                if has_test_attribute(parent, source) => {
                    return true;
                }
            "mod_item"
                if has_cfg_test_attribute(parent, source) => {
                    return true;
                }
            "source_file" | "ERROR" => return false,
            _ => {}
        }
        current = parent.parent();
    }
    false
}

fn has_test_attribute(node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut sibling = node.prev_named_sibling();
    while let Some(attr) = sibling {
        if attr.kind() != "attribute_item" {
            break;
        }
        let text = attr.utf8_text(source).unwrap_or("");
        if text.contains("#[test]")
            || text.contains("#[tokio::test]")
            || text.contains("#[async_std::test]")
        {
            return true;
        }
        sibling = attr.prev_named_sibling();
    }
    false
}

fn has_cfg_test_attribute(node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut sibling = node.prev_named_sibling();
    while let Some(attr) = sibling {
        if attr.kind() != "attribute_item" {
            break;
        }
        let text = attr.utf8_text(source).unwrap_or("");
        if text.contains("#[cfg(test)]") {
            return true;
        }
        sibling = attr.prev_named_sibling();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_rule;

    #[test]
    fn embedded_python_test_code_flagged() {
        let source = r##"
#[cfg(test)]
mod tests {
    #[test]
    fn test_lint() {
        let code = r#"
import pytest
from myapp import Calculator

def test_addition():
    calc = Calculator()
    result = calc.add(2, 3)
    assert result == 5

def test_subtraction():
    calc = Calculator()
    result = calc.subtract(5, 3)
    assert result == 2
"#;
        let result = 1; 
        assert_eq!(result, 1);
    }
}
"##;
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(!diags.is_empty());
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
        assert!(diags.is_empty());
    }

    #[test]
    fn no_test_context_ignored() {
        let source = r##"
fn helper() {
    let code = r#"
import pytest
from myapp import Calculator

def test_addition():
    calc = Calculator()
    result = calc.add(2, 3)
    assert result == 5
"#;
}
"##;
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn embedded_jest_test_code_flagged() {
        let source = r##"
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse() {
        let input = r#"
import { describe, test, expect } from 'vitest';
import { add } from './math';

describe('addition', () => {
    test('adds two numbers', () => {
        expect(add(1, 2)).toBe(3);
    });

    test('handles zero', () => {
        expect(add(0, 5)).toBe(5);
    });
});
"#;
        assert_eq!(2, 2);
    }
}
"##;
        let diags = test_rule(&StringLiteralCorpus, source);
        assert!(!diags.is_empty());
    }
}
