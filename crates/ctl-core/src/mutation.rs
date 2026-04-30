use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    Arithmetic,
    Boundary,
    NegateCondition,
    VoidReturnValue,
    ReplaceWithDefault,
    ReplaceWithLiteral,
    ReplaceOperator,
    RemoveStmt,
    UnwrapToExpect,
    Other(String),
}

impl MutationKind {
    pub fn from_cargo_mutants_name(name: &str) -> Self {
        match name {
            "replace_arithmetic_operator" => Self::Arithmetic,
            "replace_boundary" => Self::Boundary,
            "negate_condition" => Self::NegateCondition,
            "void_return_value" => Self::VoidReturnValue,
            "replace_with_default" => Self::ReplaceWithDefault,
            "replace_with_literal" => Self::ReplaceWithLiteral,
            "replace_operator" => Self::ReplaceOperator,
            "remove_statement" => Self::RemoveStmt,
            "unwrap_to_expect" => Self::UnwrapToExpect,
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivingMutant {
    pub file_path: String,
    pub line: u32,
    pub col_start: Option<u32>,
    pub col_end: Option<u32>,
    pub mutation_type: MutationKind,
    pub replacement: String,
    pub original: String,
    pub diff_hunk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationReport {
    pub mutants: Vec<SurvivingMutant>,
    pub total: usize,
    pub survived: usize,
    pub killed: usize,
    pub timeout: usize,
}

impl MutationReport {
    pub fn empty() -> Self {
        Self { mutants: Vec::new(), total: 0, survived: 0, killed: 0, timeout: 0 }
    }
}
