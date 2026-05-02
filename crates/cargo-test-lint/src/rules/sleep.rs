use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct SleepyTest;

impl Rule for SleepyTest {
    fn id(&self) -> &'static str { "CTL_SLEEP" }
    fn config_key(&self) -> &'static str { "sleepy-test" }
    fn description(&self) -> &'static str { "thread::sleep in test code" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Forbid }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
