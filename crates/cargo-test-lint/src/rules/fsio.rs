use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct FsIoInTest;

const FS_IO_METHODS: &[&str] = &[
    "std::fs::write",
    "std::fs::read",
    "std::fs::read_to_string",
    "std::fs::remove_file",
    "std::fs::create_dir",
    "std::fs::remove_dir",
    "std::fs::rename",
    "fs::write",
    "fs::read",
    "fs::read_to_string",
];

impl Rule for FsIoInTest {
    fn id(&self) -> &'static str {
        "CTL_FS_IO"
    }
    fn config_key(&self) -> &'static str {
        "fs-io-in-test"
    }
    fn description(&self) -> &'static str {
        "filesystem I/O inside test function (flakiness risk)"
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
        if !FS_IO_METHODS.contains(&func_text) {
            return vec![];
        }

        if !is_in_test_function(node, ctx.source) {
            return vec![];
        }

        vec![Diagnostic {
            rule_id: self.id().into(),
            level: self.default_level(),
            message: format!(
                "filesystem I/O `{func_text}` inside test — use tempdir guarantees or in-memory I/O"
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

fn is_in_test_function(node: tree_sitter::Node, source: &[u8]) -> bool {
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
            "source_file" => return false,
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
    fn fs_write_in_test_flagged() {
        let source = r#"
#[test]
fn test_writes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a"), "data").unwrap();
}
"#;
        let diags = test_rule(&FsIoInTest, source);
        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("write"));
    }

    #[test]
    fn fs_write_in_prod_ignored() {
        let source = r#"
pub fn save(data: &[u8]) {
    std::fs::write("output.txt", data).unwrap();
}
"#;
        let diags = test_rule(&FsIoInTest, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn short_form_fs_in_test_flagged() {
        let source = r#"
#[test]
fn test_reads() {
    let s = fs::read_to_string("file.txt").unwrap();
}
"#;
        let diags = test_rule(&FsIoInTest, source);
        assert!(!diags.is_empty());
    }

    #[test]
    fn non_fs_call_ignored() {
        let source = r#"
#[test]
fn test_math() {
    std::process::Command::new("ls").output().unwrap();
}
"#;
        let diags = test_rule(&FsIoInTest, source);
        assert!(diags.is_empty());
    }
}
