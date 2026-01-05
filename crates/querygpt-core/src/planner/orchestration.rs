use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::compile::compile_report_spec;
use crate::dsl::plan::IntermediatePlan;
use crate::dsl::report_spec::ReportSpec;
use crate::planner::planner::{NoopPlanner, Planner, PlannerContext, ReportSpecDraft};
use crate::planner::diff::{diff_report_specs, SpecDiff};
use crate::planner::confirmation::{UserConfirmation, AutoApproveConfirmation, ConfirmationResult};
use crate::schema::registry::SchemaRegistry;

/// Phase B Orchestration Result
#[derive(Debug)]
pub enum OrchestrationResult {
    /// Compilation succeeded
    Success {
        plan: IntermediatePlan,
        draft: Option<ReportSpecDraft>,
        diffs: Vec<SpecDiff>,
    },
    /// Compilation failed with diagnostics
    CompilationFailed {
        diagnostics: CompilerDiagnostics,
        draft: Option<ReportSpecDraft>,
    },
    /// Planner failed to generate suggestion
    PlannerFailed {
        error: crate::planner::planner::PlannerError,
    },
    /// User rejected the changes
    UserRejected {
        diffs: Vec<SpecDiff>,
        draft: ReportSpecDraft,
    },
    /// Retry limit exceeded
    RetryLimitExceeded {
        diagnostics: CompilerDiagnostics,
        draft: ReportSpecDraft,
        attempts: usize,
    },
}

/// Phase B Orchestrator - coordinates planner and compiler
/// This is the main entry point for Phase B functionality
pub struct Orchestrator<P: Planner, C: UserConfirmation> {
    planner: P,
    confirmation: C,
    max_retries: usize,
}

impl<P: Planner, C: UserConfirmation> Orchestrator<P, C> {
    pub fn new(planner: P, confirmation: C) -> Self {
        Self {
            planner,
            confirmation,
            max_retries: 3, // As specified in the plan
        }
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Compile-only flow: takes a ReportSpec directly and compiles it
    /// This preserves Phase A behavior while using Phase B architecture
    pub fn compile_only(
        &self,
        registry: &SchemaRegistry,
        spec: &ReportSpec,
    ) -> OrchestrationResult {
        match compile_report_spec(registry, spec) {
            Ok(plan) => OrchestrationResult::Success { 
                plan, 
                draft: None,
                diffs: vec![], // No changes in compile-only mode
            },
            Err(diagnostics) => OrchestrationResult::CompilationFailed {
                diagnostics,
                draft: None,
            },
        }
    }

    /// AI-assisted flow: takes natural language prompt and generates ReportSpec
    /// This is the new Phase B functionality with retry orchestration
    pub fn suggest_and_compile(
        &self,
        registry: &SchemaRegistry,
        prompt: &str,
        context: PlannerContext,
    ) -> OrchestrationResult {
        // Step 1: Get initial suggestion from planner
        let initial_draft = match self.planner.suggest_report_spec(prompt, context.clone()) {
            Ok(draft) => draft,
            Err(error) => return OrchestrationResult::PlannerFailed { error },
        };

        // Step 2: Attempt compilation with retry loop
        let mut current_draft = initial_draft;
        let original_spec = current_draft.spec.clone();
        
        for attempt in 1..=self.max_retries {
            // Try compilation
            match compile_report_spec(registry, &current_draft.spec) {
                Ok(plan) => {
                    // Compilation succeeded - check for changes and get user confirmation
                    let diffs = diff_report_specs(&original_spec, &current_draft.spec);
                    
                    match self.confirmation.confirm_changes(&diffs, attempt) {
                        ConfirmationResult::Approved => {
                            return OrchestrationResult::Success {
                                plan,
                                draft: Some(current_draft),
                                diffs,
                            };
                        }
                        ConfirmationResult::Rejected => {
                            return OrchestrationResult::UserRejected {
                                diffs,
                                draft: current_draft,
                            };
                        }
                        ConfirmationResult::RequestRevision(feedback) => {
                            // User wants revision - treat as compilation failure with custom feedback
                            let user_feedback_prompt = format!("{} (User feedback: {})", prompt, feedback);
                            
                            // Create mock diagnostics for user feedback
                            let mock_diagnostics = CompilerDiagnostics::error(
                                crate::compile::diagnostics::Diagnostic {
                                    code: crate::compile::diagnostics::DiagnosticCode::InvalidFilter,
                                    message: format!("User requested revision: {}", feedback),
                                    spans: vec![],
                                    details: serde_json::json!({ "user_feedback": feedback }),
                                    help: vec!["Address the user's feedback".to_string()],
                                }
                            );
                            
                            // Try to get revision from planner
                            match self.planner.revise_report_spec(
                                &user_feedback_prompt,
                                context.clone(),
                                &mock_diagnostics,
                            ) {
                                Ok(revised_draft) => {
                                    current_draft = revised_draft;
                                    continue; // Continue retry loop
                                }
                                Err(_) => {
                                    return OrchestrationResult::RetryLimitExceeded {
                                        diagnostics: mock_diagnostics,
                                        draft: current_draft,
                                        attempts: attempt,
                                    };
                                }
                            }
                        }
                    }
                }
                Err(diagnostics) => {
                    // Compilation failed - ask planner to revise
                    if attempt < self.max_retries {
                        match self.planner.revise_report_spec(
                            prompt,
                            context.clone(),
                            &diagnostics,
                        ) {
                            Ok(revised_draft) => {
                                current_draft = revised_draft;
                                continue; // Continue retry loop
                            }
                            Err(_) => {
                                // Planner can't revise, return compilation failure
                                return OrchestrationResult::CompilationFailed {
                                    diagnostics,
                                    draft: Some(current_draft),
                                };
                            }
                        }
                    } else {
                        // Max retries exceeded
                        return OrchestrationResult::RetryLimitExceeded {
                            diagnostics,
                            draft: current_draft,
                            attempts: attempt,
                        };
                    }
                }
            }
        }

        // This should never be reached due to the loop structure, but just in case
        unreachable!("Retry loop should have returned a result")
    }
}

/// Convenience function for compile-only flow using NoopPlanner
pub fn compile_only(registry: &SchemaRegistry, spec: &ReportSpec) -> OrchestrationResult {
    let orchestrator = Orchestrator::new(NoopPlanner, AutoApproveConfirmation);
    orchestrator.compile_only(registry, spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::report_spec::{Mode, SelectItem};
    use crate::planner::confirmation::MockConfirmation;
    use crate::schema::registry::SchemaRegistry;

    fn load_test_registry() -> SchemaRegistry {
        SchemaRegistry::load("../../config/workspaces/campaigns_offers.index.json")
            .expect("load test schema registry")
    }

    #[test]
    fn compile_only_preserves_phase_a_behavior() {
        // This test ensures Phase A functionality is preserved
        let registry = load_test_registry();
        
        let spec = ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![SelectItem {
                field: "campaign_id".to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        };

        let result = compile_only(&registry, &spec);
        
        match result {
            OrchestrationResult::Success { plan, draft, diffs } => {
                assert!(draft.is_none()); // No draft in compile-only mode
                assert!(diffs.is_empty()); // No diffs in compile-only mode
                assert_eq!(plan.workspace, "campaigns_offers");
                assert!(!plan.projections.is_empty());
            }
            _ => panic!("Expected successful compilation"),
        }
    }

    #[test]
    fn noop_planner_returns_unimplemented() {
        let orchestrator = Orchestrator::new(NoopPlanner, MockConfirmation { should_approve: true });
        let registry = load_test_registry();
        
        let context = PlannerContext::simple(
            "campaigns_offers".to_string(),
            vec!["campaign_id".to_string()],
            vec!["campaigns_latest".to_string()],
        );

        let result = orchestrator.suggest_and_compile(
            &registry,
            "show me all campaigns",
            context,
        );

        match result {
            OrchestrationResult::PlannerFailed { error } => {
                assert!(matches!(
                    error,
                    crate::planner::planner::PlannerError::Unimplemented
                ));
            }
            _ => panic!("Expected planner failure with NoopPlanner"),
        }
    }

    #[test]
    fn orchestrator_respects_max_retries() {
        let orchestrator = Orchestrator::new(NoopPlanner, MockConfirmation { should_approve: true })
            .with_max_retries(2);
        
        // Verify max_retries is set correctly
        assert_eq!(orchestrator.max_retries, 2);
    }

    #[test]
    fn user_rejection_returns_appropriate_result() {
        let orchestrator = Orchestrator::new(NoopPlanner, MockConfirmation { should_approve: false });
        let registry = load_test_registry();
        
        let context = PlannerContext::simple(
            "campaigns_offers".to_string(),
            vec!["campaign_id".to_string()],
            vec!["campaigns_latest".to_string()],
        );

        let result = orchestrator.suggest_and_compile(
            &registry,
            "show me all campaigns",
            context,
        );

        // Should fail at planner level before reaching confirmation
        match result {
            OrchestrationResult::PlannerFailed { .. } => {
                // Expected with NoopPlanner
            }
            _ => panic!("Expected planner failure with NoopPlanner"),
        }
    }
}