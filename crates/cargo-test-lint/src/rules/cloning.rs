use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct UnnecessaryClone;

impl Rule for UnnecessaryClone {
    fn id(&self) -> &'static str { "CTL_UNNECESSARY_CLONE" }
    fn config_key(&self) -> &'static str { "unnecessary-clone" }
    fn description(&self) -> &'static str { "unnecessary .clone()" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
