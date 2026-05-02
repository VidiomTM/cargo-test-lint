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

    #[test]
    fn parse_source_returns_tree() {
        let source = b"fn main() {}";
        let tree = parse_source(source);
        assert!(tree.is_some());
        let tree = tree.unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn parse_source_returns_none_for_invalid() {
        let source = b"fn main() {";
        let tree = parse_source(source);
        assert!(tree.is_some());
    }

    #[test]
    fn collect_rs_files_finds_rust_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("lib.rs"), "fn main() {}").unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src.join("readme.txt"), "not rust").unwrap();

        let files = collect_rs_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "rs"));
    }
}
