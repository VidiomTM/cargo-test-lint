use super::{Rule, RuleContext};
use crate::diagnostics::{Diagnostic, DiagnosticLevel};
use tree_sitter::QueryMatch;

pub struct AsyncBlocking;

impl Rule for AsyncBlocking {
    fn id(&self) -> &'static str { "CTL_ASYNC_BLOCKING" }
    fn config_key(&self) -> &'static str { "async-blocking" }
    fn description(&self) -> &'static str { "blocking call in async test" }
    fn default_level(&self) -> DiagnosticLevel { DiagnosticLevel::Warn }
    fn query_str(&self) -> &'static str { "(function_item) @fn" }
    fn validate(&self, _ctx: &RuleContext, _qm: &QueryMatch) -> Vec<Diagnostic> { vec![] }
}
