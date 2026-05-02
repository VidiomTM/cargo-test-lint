use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct TestBranching;

impl Rule for TestBranching {
    fn id(&self) -> &'static str { "CTL_TEST_BRANCHING" }
    fn config_key(&self) -> &'static str { "test-branching" }
    fn description(&self) -> &'static str { "control flow in test body" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
