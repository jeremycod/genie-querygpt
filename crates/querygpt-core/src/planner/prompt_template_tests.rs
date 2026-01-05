use crate::planner::{
    prompt_templates::PromptTemplates,
    planner::{PlannerContext, Planner},
    llm_planner::LlmPlanner,
    mock_client::MockClient,
};
use crate::dsl::report_spec::{ReportSpec, SelectItem, Mode};
use crate::compile::diagnostics::CompilerDiagnostics;

#[test]
fn system_prompt_includes_required_elements() {
    let ctx = PlannerContext {
        workspace: "test_workspace".to_string(),
        available_fields: vec!["offer_id".to_string(), "region".to_string()],
        available_tables: vec!["offers".to_string()],
    };
    
    let prompt = PromptTemplates::system_prompt(&ctx);
    
    assert!(prompt.contains("test_workspace"));
    assert!(prompt.contains("offer_id, region"));
    assert!(prompt.contains("offers"));
    assert!(prompt.contains("REQUIRED OUTPUT FORMAT"));
    assert!(prompt.contains("Only output valid JSON"));
}

#[test]
fn revision_prompt_includes_error_context() {
    let ctx = PlannerContext {
        workspace: "test".to_string(),
        available_fields: vec!["offer_id".to_string()],
        available_tables: vec!["offers".to_string()],
    };
    
    let spec = ReportSpec {
        version: 1,
        workspace: "test".to_string(),
        select: vec![SelectItem {
            field: "invalid_field".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };
    
    let diagnostics = CompilerDiagnostics::new();
    let original_prompt = "test prompt";
    
    let prompt = PromptTemplates::revision_prompt(original_prompt, &spec, &diagnostics, &ctx);
    
    assert!(prompt.contains("Previous attempt failed"));
    assert!(prompt.contains("test prompt"));
    assert!(prompt.contains("invalid_field"));
    assert!(prompt.contains("Fix the errors"));
}

#[test]
fn user_prompt_formats_natural_language() {
    let nl = "Show me all offers in APAC";
    let prompt = PromptTemplates::user_prompt(nl);
    
    assert!(prompt.contains("Generate a ReportSpec"));
    assert!(prompt.contains("Show me all offers in APAC"));
}

#[test]
fn json_parsing_handles_valid_response() {
    let mock_client = MockClient::new()
        .with_default_response(r#"{
            "report_spec": {
                "version": 1,
                "workspace": "test",
                "select": [{"field": "offer_id", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Test assumption"],
            "open_questions": [],
            "notes": "Test response"
        }"#.to_string());
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    
    let ctx = PlannerContext {
        workspace: "test".to_string(),
        available_fields: vec!["offer_id".to_string()],
        available_tables: vec!["offers".to_string()],
    };
    
    let result = planner.suggest_report_spec("test", ctx);
    assert!(result.is_ok());
}

#[test]
fn json_parsing_rejects_invalid_response() {
    let mock_client = MockClient::new()
        .with_default_response("This is not JSON".to_string());
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    
    let ctx = PlannerContext {
        workspace: "test".to_string(),
        available_fields: vec!["offer_id".to_string()],
        available_tables: vec!["offers".to_string()],
    };
    
    let result = planner.suggest_report_spec("test", ctx);
    assert!(result.is_err());
}

#[test]
fn json_parsing_extracts_json_from_mixed_content() {
    let mock_client = MockClient::new()
        .with_default_response(r#"Here's the JSON response:
        {
            "report_spec": {
                "version": 1,
                "workspace": "test",
                "select": [{"field": "offer_id", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Extracted from mixed content"],
            "open_questions": [],
            "notes": "Test"
        }
        That's the response."#.to_string());
    
    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());
    
    let ctx = PlannerContext {
        workspace: "test".to_string(),
        available_fields: vec!["offer_id".to_string()],
        available_tables: vec!["offers".to_string()],
    };
    
    let result = planner.suggest_report_spec("test", ctx);
    assert!(result.is_ok());
    assert!(result.unwrap().assumptions.contains(&"Extracted from mixed content".to_string()));
}