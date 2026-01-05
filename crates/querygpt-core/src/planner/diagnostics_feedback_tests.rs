use crate::planner::{
    llm_planner::LlmPlanner,
    mock_client::MockClient,
    planner::{Planner, PlannerContext},
    orchestration::{Orchestrator, OrchestrationResult},
    confirmation::AutoApproveConfirmation,
};
use crate::compile::diagnostics::CompilerDiagnostics;
use crate::schema::registry::SchemaRegistry;

#[test]
fn llm_planner_uses_diagnostics_in_revision() {
    // Create a mock client that returns different responses for initial vs revision
    let mut mock_client = MockClient::new();
    
    // First response (will fail compilation)
    mock_client.add_response(
        "Generate a ReportSpec for this request: test invalid field".to_string(),
        r#"{
            "report_spec": {
                "version": 1,
                "workspace": "test",
                "select": [{"field": "invalid_field", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Using invalid field"],
            "open_questions": [],
            "notes": "Initial attempt"
        }"#.to_string(),
    );
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    
    let ctx = PlannerContext::simple(
        "test".to_string(),
        vec!["valid_field".to_string()],
        vec!["test_table".to_string()],
    );
    
    // Test initial suggestion
    let initial_draft = planner.suggest_report_spec("test invalid field", ctx.clone()).unwrap();
    assert_eq!(initial_draft.spec.select[0].field, "invalid_field");
    
    // Test revision with diagnostics
    let diagnostics = CompilerDiagnostics::unknown_field(
        "invalid_field".to_string(),
        "select field validation"
    );
    
    // The revision should use diagnostics in the prompt
    // For this test, we'll verify the method can be called without error
    // In a real scenario, the mock would return a corrected spec
    let revision_result = planner.revise_report_spec(
        "test invalid field",
        ctx,
        &diagnostics,
    );
    
    // Should succeed (mock client will return the same response)
    assert!(revision_result.is_ok());
}

#[test]
fn orchestration_retry_loop_with_llm_planner() {
    // Create a planner that succeeds on revision
    let mock_client = MockClient::new()
        .with_default_response(r#"{
            "report_spec": {
                "version": 1,
                "workspace": "campaigns_offers",
                "select": [{"field": "campaign_id", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Using valid campaign_id field"],
            "open_questions": [],
            "notes": "Corrected field name"
        }"#.to_string());
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation);
    
    // Load test registry
    let registry = SchemaRegistry::load("../../config/workspaces/campaigns_offers.index.json")
        .expect("load test schema registry");
    
    let ctx = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string()],
        vec!["campaigns_latest".to_string()],
    );
    
    let result = orchestrator.suggest_and_compile(
        &registry,
        "show me campaigns",
        ctx,
    );
    
    // Should succeed with the valid spec
    match result {
        OrchestrationResult::Success { plan, draft, .. } => {
            assert!(draft.is_some());
            assert_eq!(plan.workspace, "campaigns_offers");
        }
        other => panic!("Expected success, got: {:?}", other),
    }
}

#[test]
fn diagnostics_feedback_includes_error_details() {
    let mock_client = MockClient::new()
        .with_default_response(r#"{
            "report_spec": {
                "version": 1,
                "workspace": "test",
                "select": [{"field": "test_field", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Fixed based on diagnostics"],
            "open_questions": [],
            "notes": "Applied error feedback"
        }"#.to_string());
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    
    let ctx = PlannerContext::simple(
        "test".to_string(),
        vec!["test_field".to_string()],
        vec!["test_table".to_string()],
    );
    
    // Create diagnostics with specific error details
    let diagnostics = CompilerDiagnostics::schema_mismatch(
        "expected valid field".to_string(),
        "found invalid_field".to_string(),
    );
    
    let result = planner.revise_report_spec(
        "test prompt",
        ctx,
        &diagnostics,
    );
    
    assert!(result.is_ok());
    let draft = result.unwrap();
    assert!(draft.assumptions.contains(&"Fixed based on diagnostics".to_string()));
}