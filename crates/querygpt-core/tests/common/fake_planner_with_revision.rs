// Used in orchestration_comprehensive_tests.rs
#![allow(dead_code)]

use querygpt_core::compile::diagnostics::CompilerDiagnostics;
use querygpt_core::dsl::report_spec::ReportSpec;
use querygpt_core::planner::planner::{Planner, PlannerContext, PlannerResult, ReportSpecDraft};
use std::sync::{Arc, Mutex};

/// FakePlannerWithRevision - Test helper for orchestration revision flow
/// Returns an invalid spec on first attempt, valid spec on revision
/// Used for testing the diagnostics feedback loop
pub struct FakePlannerWithRevision {
    first_spec: ReportSpec,
    revised_spec: ReportSpec,
    revision_called: Arc<Mutex<bool>>,
}

impl FakePlannerWithRevision {
    pub fn new(first_spec: ReportSpec, revised_spec: ReportSpec) -> Self {
        Self {
            first_spec,
            revised_spec,
            revision_called: Arc::new(Mutex::new(false)),
        }
    }

    pub fn was_revision_called(&self) -> bool {
        *self.revision_called.lock().unwrap()
    }
}

#[async_trait::async_trait]
impl Planner for FakePlannerWithRevision {
    async fn suggest_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        Ok(ReportSpecDraft {
            spec: self.first_spec.clone(),
            rationale: Some("Initial suggestion (intentionally invalid for testing)".to_string()),
            assumptions: vec!["This spec will fail compilation".to_string()],
        })
    }

    async fn revise_report_spec(
        &self,
        _prompt: &str,
        _ctx: PlannerContext,
        _diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        *self.revision_called.lock().unwrap() = true;
        Ok(ReportSpecDraft {
            spec: self.revised_spec.clone(),
            rationale: Some("Revised suggestion (corrected based on diagnostics)".to_string()),
            assumptions: vec!["Fixed compilation errors".to_string()],
        })
    }
}
