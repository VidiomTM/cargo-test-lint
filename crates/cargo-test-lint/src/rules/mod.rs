pub mod assertions;
pub mod async_safety;
pub mod cloning;
pub mod complexity;
pub mod dead_code;
pub mod flow;
pub mod nextest;
pub mod sleep;
pub mod structure;
pub mod drop;

use crate::config::Config;
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Query, QueryCursor, QueryMatch, Tree};

pub struct RuleContext<'a> {
    pub source: &'a [u8],
    pub tree: &'a Tree,
    pub config: &'a Config,
    pub file_path: &'a Path,
}

pub trait Rule {
    fn id(&self) -> &'static str;
    fn config_key(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn default_level(&self) -> DiagnosticLevel;
    fn query_str(&self) -> &'static str;
    fn validate(&self, ctx: &RuleContext, query_match: &QueryMatch) -> Vec<Diagnostic>;
}

pub fn run_rule<'a>(rule: &dyn Rule, ctx: &RuleContext<'a>) -> Vec<Diagnostic> {
    let level = ctx.config.rule_level(rule.config_key(), rule.default_level());
    if level == DiagnosticLevel::Allow {
        return vec![];
    }

    let language = tree_sitter_rust::LANGUAGE.into();
    let Ok(query) = Query::new(&language, rule.query_str()) else {
        return vec![];
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, ctx.tree.root_node(), ctx.source);

    let mut diagnostics = Vec::new();
    while let Some(query_match) = matches.next() {
        let mut rule_diags = rule.validate(ctx, &query_match);
        for diag in &mut rule_diags {
            diag.level = level.clone();
        }
        diagnostics.extend(rule_diags);
    }

    diagnostics
}

pub fn run_all_rules(ctx: &RuleContext) -> Vec<Diagnostic> {
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(assertions::AssertMsg),
        Box::new(assertions::MaxExpects),
        Box::new(sleep::SleepyTest),
        Box::new(flow::TestBranching),
        Box::new(nextest::StaticMut),
        Box::new(nextest::EnvSetVar),
        Box::new(async_safety::AsyncBlocking),
        Box::new(structure::NestedMod),
        Box::new(cloning::UnnecessaryClone),
        Box::new(complexity::DeepWrapper),
        Box::new(drop::MissingDropGuard),
        Box::new(dead_code::DeadTestHelper),
    ];

    let mut diagnostics = Vec::new();
    for rule in &rules {
        if ctx.config.rule_enabled(rule.config_key()) {
            diagnostics.extend(run_rule(rule.as_ref(), ctx));
        }
    }
    Diagnostic::sort_by_position(&mut diagnostics);
    diagnostics
}

/// Helper for tests: parse a snippet and run a single rule, returning diagnostics.
#[cfg(test)]
pub fn test_rule(rule: &dyn Rule, source: &str) -> Vec<Diagnostic> {
    let tree = crate::parser::parse_source(source.as_bytes()).unwrap();
    let config = Config::default();
    let ctx = RuleContext {
        source: source.as_bytes(),
        tree: &tree,
        config: &config,
        file_path: Path::new("test.rs"),
    };
    run_rule(rule, &ctx)
}

/// Helper for tests: parse a snippet and run a single rule with custom config.
#[cfg(test)]
pub fn test_rule_with_config(
    rule: &dyn Rule,
    source: &str,
    config: Config,
) -> Vec<Diagnostic> {
    let tree = crate::parser::parse_source(source.as_bytes()).unwrap();
    let ctx = RuleContext {
        source: source.as_bytes(),
        tree: &tree,
        config: &config,
        file_path: Path::new("test.rs"),
    };
    run_rule(rule, &ctx)
}
