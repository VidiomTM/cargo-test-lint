use tree_sitter::Node;

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

fn has_cfg_test_attribute(node: Node, source: &[u8]) -> bool {
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
