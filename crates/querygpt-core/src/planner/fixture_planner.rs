use super::planner::{Planner, PlannerContext, PlannerResult, PlannerError, ReportSpecDraft};
use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::report_spec::ReportSpec;
use std::collections::HashMap;

/// Fixture-based planner that maps known prompts to known specs
/// Used for deterministic testing without LLM dependency
pub struct FixturePlanner {
    fixtures: HashMap<String, ReportSpec>,
}

impl FixturePlanner {
    pub fn new() -> Self {
        Self {
            fixtures: HashMap::new(),
        }
    }

    pub fn with_fixture(mut self, prompt: String, spec: ReportSpec) -> Self {
        self.fixtures.insert(prompt, spec);
        self
    }

    pub fn add_fixture(&mut self, prompt: String, spec: ReportSpec) {
        self.fixtures.insert(prompt, spec);
    }
}

impl Default for FixturePlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner for FixturePlanner {
    fn suggest_report_spec(
        &self,
        prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        match self.fixtures.get(prompt) {
            Some(spec) => Ok(ReportSpecDraft {
                spec: spec.clone(),
                rationale: Some(format!("Fixture response for prompt: {}", prompt)),
                assumptions: vec!["Using fixture data".to_string()],
            }),
            None => Err(PlannerError::InvalidPrompt(format!(
                "No fixture found for prompt: {}",
                prompt
            ))),
        }
    }

    fn revise_report_spec(
        &self,
        prompt: &str,
        _ctx: PlannerContext,
        _diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        // For fixture planner, revision is the same as initial suggestion
        // In real scenarios, we might have separate revision fixtures
        self.suggest_report_spec(prompt, _ctx)
    }
}