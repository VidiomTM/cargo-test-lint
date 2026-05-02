use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeadTestHelper;

impl Rule for DeadTestHelper {
    fn id(&self) -> &'static str { "CTL_DEAD_TEST_HELPER" }
    fn config_key(&self) -> &'static str { "dead-test-helper" }
    fn description(&self) -> &'static str { "unused test helper" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
