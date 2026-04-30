use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::warn;

use ctl_core::mutation::{MutationKind, MutationReport, SurvivingMutant};

#[derive(Debug, Deserialize)]
struct LabOutcome {
    outcomes: Vec<ScenarioOutcome>,
    total_mutants: usize,
    missed: usize,
    caught: usize,
    timeout: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScenarioOutcome {
    scenario: Scenario,
    summary: SummaryOutcome,
    log_path: Option<String>,
    diff_path: Option<String>,
    phase_results: Vec<PhaseResult>,
}

#[derive(Debug, Deserialize)]
enum Scenario {
    #[serde(rename = "baseline")]
    Baseline,
    #[serde(rename = "mutant")]
    Mutant(MutantInfo),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SummaryOutcome {
    Success,
    CaughtMutant,
    MissedMutant,
    Unviable,
    Failure,
    Timeout,
}

#[derive(Debug, Deserialize)]
struct MutantInfo {
    name: String,
    #[allow(dead_code)]
    package: String,
    file: String,
    function: Option<FunctionInfo>,
    span: SpanInfo,
    replacement: String,
    genre: GenreInfo,
}

#[derive(Debug, Deserialize)]
struct FunctionInfo {
    function_name: String,
    #[allow(dead_code)]
    return_type: String,
    #[allow(dead_code)]
    span: SpanInfo,
}

#[derive(Debug, Deserialize)]
struct SpanInfo {
    start: LineCol,
    end: LineCol,
}

#[derive(Debug, Deserialize)]
struct LineCol {
    line: u32,
    column: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GenreInfo {
    FnValue,
    BinaryOperator,
    UnaryOperator,
    MatchArm,
    MatchArmGuard,
    StructField,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PhaseResult {
    phase: String,
    duration: f64,
    process_status: serde_json::Value,
    argv: Vec<String>,
}

pub fn parse_outcomes(outcomes_json: &str, mutants_dir: &Path) -> Result<MutationReport> {
    let lab: LabOutcome =
        serde_json::from_str(outcomes_json).context("Failed to parse outcomes.json")?;

    let mut mutants = Vec::new();

    for outcome in &lab.outcomes {
        let Scenario::Mutant(ref info) = outcome.scenario else {
            continue;
        };

        let is_survived = matches!(outcome.summary, SummaryOutcome::MissedMutant);
        if !is_survived {
            continue;
        }

        let mutation_type = genre_to_mutation_kind(&info.genre, &info.name);

        let diff_hunk = outcome.diff_path.as_ref().and_then(|dp| {
            let diff_file = mutants_dir.join(dp);
            std::fs::read_to_string(&diff_file).ok()
        });

        mutants.push(SurvivingMutant {
            file_path: info.file.clone(),
            line: info.span.start.line,
            col_start: Some(info.span.start.column),
            col_end: Some(info.span.end.column),
            mutation_type,
            replacement: info.replacement.clone(),
            original: extract_original(info),
            diff_hunk,
        });
    }

    let total = lab.total_mutants;
    let survived = lab.missed;
    let killed = lab.caught;
    let timeout = lab.timeout;

    if survived != mutants.len() {
        warn!(
            "outcomes.json reports {survived} missed but parsed {} surviving mutants",
            mutants.len()
        );
    }

    Ok(MutationReport { mutants, total, survived, killed, timeout })
}

fn genre_to_mutation_kind(genre: &GenreInfo, _name: &str) -> MutationKind {
    match genre {
        GenreInfo::FnValue => MutationKind::VoidReturnValue,
        GenreInfo::BinaryOperator => MutationKind::ReplaceOperator,
        GenreInfo::UnaryOperator => MutationKind::ReplaceOperator,
        GenreInfo::MatchArm => MutationKind::RemoveStmt,
        GenreInfo::MatchArmGuard => MutationKind::ReplaceWithLiteral,
        GenreInfo::StructField => MutationKind::RemoveStmt,
    }
}

fn extract_original(info: &MutantInfo) -> String {
    if let Some(func) = &info.function {
        if matches!(info.genre, GenreInfo::FnValue) {
            return format!("fn {}() {{ ... }}", func.function_name);
        }
    }
    String::new()
}

pub fn parse_outcomes_from_dir(mutants_dir: &Path) -> Result<MutationReport> {
    let outcomes_path = mutants_dir.join("outcomes.json");
    let json = std::fs::read_to_string(&outcomes_path)
        .with_context(|| format!("Failed to read {}", outcomes_path.display()))?;
    parse_outcomes(&json, mutants_dir)
}
