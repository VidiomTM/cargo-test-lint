use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct DeepWrapper;

impl Rule for DeepWrapper {
    fn id(&self) -> &'static str { "CTL_DEEP_WRAPPER" }
    fn config_key(&self) -> &'static str { "deep-wrapper" }
    fn description(&self) -> &'static str { "deeply nested type wrapper" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(type_item) @type" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
