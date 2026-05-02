use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct StaticMut;
pub struct EnvSetVar;

impl Rule for StaticMut {
    fn id(&self) -> &'static str { "CTL_STATIC_MUT" }
    fn description(&self) -> &'static str { "static mutable variable" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(static_item) @static" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}

impl Rule for EnvSetVar {
    fn id(&self) -> &'static str { "CTL_ENV_SET_VAR" }
    fn description(&self) -> &'static str { "std::env::set_var in test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(call_expression) @call" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
