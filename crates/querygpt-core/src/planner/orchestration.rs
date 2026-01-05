use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::compile::compile_report_spec;
use crate::dsl::plan::IntermediatePlan;
use crate::dsl::report_spec::ReportSpec;
use crate::planner::planner::{NoopPlanner, Planner, PlannerContext, ReportSpecDraft};
use crate::schema::registry::SchemaRegistry;

/// Phase B Orchestration Result
#[derive(Debug)]
pub enum OrchestrationResult {
    /// Compilation succeeded
    Success {
        plan: IntermediatePlan,
        draft: Option<ReportSpecDraft>,
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
}

/// Phase B Orchestrator - coordinates planner and compiler
/// This is the main entry point for Phase B functionality
pub struct Orchestrator<P: Planner> {
    planner: P,
    max_retries: usize,
}

impl<P: Planner> Orchestrator<P> {
    pub fn new(planner: P) -> Self {
        Self {
            planner,
            max_retries: 3, // As specified in the plan
        }
    }

    /// Compile-only flow: takes a ReportSpec directly and compiles it
    /// This preserves Phase A behavior while using Phase B architecture
    pub fn compile_only(
        &self,
        registry: &SchemaRegistry,
        spec: &ReportSpec,
    ) -> OrchestrationResult {
        match compile_report_spec(registry, spec) {
            Ok(plan) => OrchestrationResult::Success { plan, draft: None },
            Err(diagnostics) => OrchestrationResult::CompilationFailed {
                diagnostics,
                draft: None,
            },
        }
    }

    /// AI-assisted flow: takes natural language prompt and generates ReportSpec
    /// This is the new Phase B functionality (currently using NoopPlanner)
    pub fn suggest_and_compile(
        &self,
        registry: &SchemaRegistry,
        prompt: &str,
        context: PlannerContext,
    ) -> OrchestrationResult {
        // Step 1: Get initial suggestion from planner
        let draft = match self.planner.suggest_report_spec(prompt, context.clone()) {
            Ok(draft) => draft,
            Err(error) => return OrchestrationResult::PlannerFailed { error },
        };

        // Step 2: Attempt compilation with retry loop
        let mut current_draft = draft;
        for _attempt in 0..self.max_retries {
            match compile_report_spec(registry, &current_draft.spec) {
                Ok(plan) => {
                    return OrchestrationResult::Success {
                        plan,
                        draft: Some(current_draft),
                    }
                }
                Err(diagnostics) => {
                    // Step 3: If compilation fails, ask planner to revise
                    match self.planner.revise_report_spec(
                        prompt,
                        context.clone(),
                        &diagnostics,
                    ) {
                        Ok(revised_draft) => {
                            current_draft = revised_draft;
                            // Continue retry loop
                        }
                        Err(_) => {
                            // Planner can't revise, return compilation failure
                            return OrchestrationResult::CompilationFailed {
                                diagnostics,
                                draft: Some(current_draft),
                            };
                        }
                    }
                }
            }
        }

        // Max retries exceeded
        // Try one final compilation to get the latest diagnostics
        match compile_report_spec(registry, &current_draft.spec) {
            Ok(plan) => OrchestrationResult::Success {
                plan,
                draft: Some(current_draft),
            },
            Err(diagnostics) => OrchestrationResult::CompilationFailed {
                diagnostics,
                draft: Some(current_draft),
            },
        }
    }
}

/// Convenience function for compile-only flow using NoopPlanner
pub fn compile_only(registry: &SchemaRegistry, spec: &ReportSpec) -> OrchestrationResult {
    let orchestrator = Orchestrator::new(NoopPlanner);
    orchestrator.compile_only(registry, spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::report_spec::{Mode, SelectItem};
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
            OrchestrationResult::Success { plan, draft } => {
                assert!(draft.is_none()); // No draft in compile-only mode
                assert_eq!(plan.workspace, "campaigns_offers");
                assert!(!plan.projections.is_empty());
            }
            _ => panic!("Expected successful compilation"),
        }
    }

    #[test]
    fn noop_planner_returns_unimplemented() {
        let orchestrator = Orchestrator::new(NoopPlanner);
        let registry = load_test_registry();
        
        let context = PlannerContext {
            workspace: "campaigns_offers".to_string(),
            available_fields: vec!["campaign_id".to_string()],
            available_tables: vec!["campaigns_latest".to_string()],
        };

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
}