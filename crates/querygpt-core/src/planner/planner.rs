use super::schema_summary::{ExamplePair, PlannerConstraints, SchemaSummary};
use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::report_spec::ReportSpec;
use serde::{Deserialize, Serialize};

/// Phase B Planner Interface - Strict boundary for AI behavior
/// AI output is treated as untrusted and must go through compiler
#[async_trait::async_trait]
pub trait Planner {
    async fn suggest_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft>;

    async fn revise_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
        diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft>;
}

/// Enhanced context provided to planner for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContext {
    pub workspace: String,
    pub schema_summary: SchemaSummary,
    pub report_spec_schema: Option<serde_json::Value>,
    pub examples: Vec<ExamplePair>,
    pub constraints: PlannerConstraints,
    /// Current date in YYYY-MM-DD format for "today" queries
    #[serde(default)]
    pub current_date: Option<String>,
    // Legacy fields for backward compatibility
    #[serde(default)]
    pub available_fields: Vec<String>,
    #[serde(default)]
    pub available_tables: Vec<String>,
}

impl PlannerContext {
    /// Create a simple context for testing (backward compatible)
    pub fn simple(
        workspace: String,
        available_fields: Vec<String>,
        available_tables: Vec<String>,
    ) -> Self {
        let schema_summary = SchemaSummary::minimal(&workspace);
        Self {
            workspace: workspace.clone(),
            schema_summary,
            report_spec_schema: None,
            examples: vec![],
            constraints: PlannerConstraints::default(),
            current_date: None,
            available_fields,
            available_tables,
        }
    }

    /// Create an enhanced context with full schema summary
    pub fn enhanced(
        workspace: String,
        schema_summary: SchemaSummary,
        examples: Vec<ExamplePair>,
        constraints: Option<PlannerConstraints>,
    ) -> Self {
        let available_fields = schema_summary.get_all_fields();
        let available_tables = schema_summary.get_all_tables();

        Self {
            workspace,
            schema_summary,
            report_spec_schema: None,
            examples,
            constraints: constraints.unwrap_or_default(),
            current_date: None,
            available_fields,
            available_tables,
        }
    }

    /// Set the current date for "today" queries
    pub fn with_current_date(mut self, date: String) -> Self {
        self.current_date = Some(date);
        self
    }
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

#[async_trait::async_trait]
impl Planner for NoopPlanner {
    async fn suggest_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }

    async fn revise_report_spec(
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

#[async_trait::async_trait]
impl Planner for StubPlanner {
    async fn suggest_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }

    async fn revise_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
        _diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        Err(PlannerError::Unimplemented)
    }
}
