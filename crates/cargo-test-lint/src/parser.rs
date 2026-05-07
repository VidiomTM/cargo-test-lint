use std::path::Path;
use tree_sitter::{Parser, Tree};

pub fn make_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("failed to set tree-sitter-rust language");
    parser
}

pub fn parse_source(source: &[u8]) -> Option<Tree> {
    let mut parser = make_parser();
    parser.parse(source, None)
}

pub fn parse_file(path: &Path) -> anyhow::Result<(Vec<u8>, Tree)> {
    let source = std::fs::read(path)?;
    let tree = parse_source(&source)
        .ok_or_else(|| anyhow::anyhow!("failed to parse {}", path.display()))?;
    Ok((source, tree))
}

pub fn collect_rs_files(project_root: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in ignore::Walk::new(project_root) {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn write_file(path: &Path, content: &str) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn parse_source_returns_tree() {
        let source = b"fn main() {}";
        let tree = parse_source(source);
        assert!(tree.is_some(), "valid source should parse to a tree");
        let tree = tree.unwrap();
        assert_eq!(tree.root_node().kind(), "source_file", "root node should be source_file");
    }

    #[test]
    fn parse_source_returns_tree_even_for_invalid() {
        // tree-sitter always returns a tree with ERROR nodes for invalid input
        let source = b"fn main() {";
        let tree = parse_source(source);
        assert!(tree.is_some(), "tree-sitter should return a tree even for invalid input");
    }

    #[test]
    fn collect_rs_files_finds_rust_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        write_file(&src.join("lib.rs"), "fn main() {}");
        write_file(&src.join("main.rs"), "fn main() {}");
        write_file(&src.join("readme.txt"), "not rust");

        let files = collect_rs_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2, "should find exactly 2 .rs files");
        assert!(
            files.iter().all(|p| p.extension().unwrap() == "rs"),
            "all found files should have .rs extension"
        );
    }
}
