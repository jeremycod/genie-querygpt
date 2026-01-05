use crate::dsl::report_spec::ReportSpec;
use crate::compile::diagnostics::CompilerDiagnostics;
use serde::{Deserialize, Serialize};

/// Phase B Planner Interface - Strict boundary for AI behavior
/// AI output is treated as untrusted and must go through compiler
pub trait Planner {
    fn suggest_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft>;

    fn revise_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
        diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft>;
}

/// Context provided to planner for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContext {
    pub workspace: String,
    pub available_fields: Vec<String>,
    pub available_tables: Vec<String>,
}

/// Planner output - always a draft that must be compiled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSpecDraft {
    pub spec: ReportSpec,
    pub rationale: Option<String>,
    pub assumptions: Vec<String>,
}

/// Result type for planner operations
pub type PlannerResult<T> = Result<T, PlannerError>;

#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("planner functionality is not implemented yet")]
    Unimplemented,

    #[error("invalid planning prompt: {0}")]
    InvalidPrompt(String),

    #[error("planner internal error: {0}")]
    InternalError(String),
}

/// NoopPlanner - Does nothing, used for compile-only flow
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPlanner;

impl Planner for NoopPlanner {
    fn suggest_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }

    fn revise_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
        _diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }
}

/// Legacy StubPlanner - kept for backward compatibility during transition
#[derive(Debug, Default, Clone, Copy)]
pub struct StubPlanner;

impl Planner for StubPlanner {
    fn suggest_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }

    fn revise_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
        _diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }
}
