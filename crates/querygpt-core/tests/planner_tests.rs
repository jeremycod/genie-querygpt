use querygpt_core::dsl::report_spec::{Mode, ReportSpec, SelectItem};
use querygpt_core::planner::{
    fixture_planner::FixturePlanner,
    llm_planner::LlmPlanner,
    mock_client::MockClient,
    planner::{Planner, PlannerContext},
};

#[tokio::test]
async fn fixture_planner_returns_configured_spec() {
    let mut planner = FixturePlanner::new();

    let test_spec = ReportSpec {
        version: 1,
        workspace: "test".to_string(),
        select: vec![SelectItem {
            field: "offer_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    planner.add_fixture("test prompt".to_string(), test_spec.clone());

    let ctx = PlannerContext::simple(
        "test".to_string(),
        vec!["offer_id".to_string()],
        vec!["offers".to_string()],
    );

    let result = planner
        .suggest_report_spec("test prompt", ctx)
        .await
        .unwrap();
    assert_eq!(result.spec, test_spec);
    assert!(result
        .assumptions
        .contains(&"Using fixture data".to_string()));
}

#[tokio::test]
async fn fixture_planner_returns_error_for_unknown_prompt() {
    let planner = FixturePlanner::new();

    let ctx = PlannerContext::simple("test".to_string(), vec![], vec![]);

    let result = planner.suggest_report_spec("unknown prompt", ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn mock_client_returns_configured_response() {
    let mock_client = MockClient::new().with_default_response(
        r#"{
            "report_spec": {
                "version": 1,
                "workspace": "test",
                "select": [{"field": "offer_id", "alias": null}],
                "filters": [],
                "order_by": [],
                "mode": "preview",
                "pagination": null
            },
            "assumptions": ["Mock response"],
            "open_questions": [],
            "notes": "Test response"
        }"#
        .to_string(),
    );

    let planner = LlmPlanner::new(Box::new(mock_client), "test-model".to_string());

    let ctx = PlannerContext::simple(
        "test".to_string(),
        vec!["offer_id".to_string()],
        vec!["offers".to_string()],
    );

    let result = planner
        .suggest_report_spec("test prompt", ctx)
        .await
        .unwrap();
    assert_eq!(result.spec.select.len(), 1);
    assert_eq!(result.spec.select[0].field, "offer_id");
    assert!(result.assumptions.contains(&"Mock response".to_string()));
}
