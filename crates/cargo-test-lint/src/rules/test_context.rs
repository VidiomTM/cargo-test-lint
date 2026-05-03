use regex::Regex;
use std::sync::LazyLock;
use tree_sitter::Node;

static TEST_ATTR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\s*\[\s*(?:tokio::test|async_std::test|test)(?:\s*\([^)]*\))?\s*\]").unwrap()
});

static CFG_TEST_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#\s*\[\s*cfg\s*\(\s*[^)]*test[^)]*\)\s*\]").unwrap()
});

pub fn is_in_test_function(node: Node, source: &[u8]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "function_item" | "function_signature" if has_test_attribute(parent, source) => {
                return true;
            }
            "mod_item" if has_cfg_test_attribute(parent, source) => {
                return true;
            }
            "source_file" | "ERROR" => return false,
            _ => {}
        }
        current = parent.parent();
    }
    false
}

fn has_test_attribute(node: Node, source: &[u8]) -> bool {
    let mut sibling = node.prev_named_sibling();
    while let Some(attr) = sibling {
        if attr.kind() != "attribute_item" {
            break;
        }
        let text = attr.utf8_text(source).unwrap_or("");
        if TEST_ATTR_REGEX.is_match(text) {
            return true;
        }
        sibling = attr.prev_named_sibling();
    }
    false
}

fn has_cfg_test_attribute(node: Node, source: &[u8]) -> bool {
    let mut sibling = node.prev_named_sibling();
    while let Some(attr) = sibling {
        if attr.kind() != "attribute_item" {
            break;
        }
        let text = attr.utf8_text(source).unwrap_or("");
        if CFG_TEST_REGEX.is_match(text) {
            return true;
        }
        sibling = attr.prev_named_sibling();
    }
    false
}
