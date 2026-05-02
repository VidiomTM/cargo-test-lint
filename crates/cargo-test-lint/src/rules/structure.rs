use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct NestedMod;

impl Rule for NestedMod {
    fn id(&self) -> &'static str { "CTL_NESTED_MOD" }
    fn config_key(&self) -> &'static str { "nested-mod" }
    fn description(&self) -> &'static str { "deeply nested test module" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(mod_item) @mod" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
