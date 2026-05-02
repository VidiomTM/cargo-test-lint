use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct MissingDropGuard;

impl Rule for MissingDropGuard {
    fn id(&self) -> &'static str { "CTL_MISSING_DROP_GUARD" }
    fn description(&self) -> &'static str { "resource allocation without RAII guard" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
