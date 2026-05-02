use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AssertMsg;
pub struct MaxExpects;

impl Rule for AssertMsg {
    fn id(&self) -> &'static str { "CTL_ASSERT_MSG" }
    fn description(&self) -> &'static str { "assertion missing context message" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(macro_invocation) @macro" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}

impl Rule for MaxExpects {
    fn id(&self) -> &'static str { "CTL_MAX_EXPECTS" }
    fn description(&self) -> &'static str { "too many assertions in test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
