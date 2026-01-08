use querygpt_core::dsl::report_spec::{Filter, FilterOp, Mode, ReportSpec, SelectItem};
use querygpt_core::planner::{
    confirmation::AutoApproveConfirmation,
    fixture_planner::FixturePlanner,
    orchestration::{OrchestrationResult, Orchestrator},
    planner::PlannerContext,
};
use querygpt_core::schema::registry::SchemaRegistry;

mod common;
use common::FakePlannerWithRevision;

fn load_test_registry() -> SchemaRegistry {
    // Use absolute path to avoid working directory issues with concurrent tests
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(|p| p.parent())
        .expect("resolve repo root from CARGO_MANIFEST_DIR");
    let index_path = repo_root.join("config/workspaces/campaigns_offers.index.json");

    SchemaRegistry::load(index_path.to_str().unwrap()).expect("load test schema registry")
}

#[tokio::test]
async fn orchestration_success_path_with_fixture_planner() {
    // Create a valid spec fixture
    let valid_spec = ReportSpec {
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

    let mut planner = FixturePlanner::new();
    planner.add_fixture("show me campaigns".to_string(), valid_spec.clone());

    let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation);

    // Load test registry
    let registry = load_test_registry();

    let ctx = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string()],
        vec!["campaigns_latest".to_string()],
    );

    let result = orchestrator
        .suggest_and_compile(&registry, "show me campaigns", ctx)
        .await;

    // Should succeed with valid spec
    match result {
        OrchestrationResult::Success {
            plan,
            draft,
            diffs: _,
            trace,
        } => {
            assert!(draft.is_some());
            assert_eq!(plan.workspace, "campaigns_offers");
            assert!(trace.is_some());
        }
        other => panic!("Expected success, got: {:?}", other),
    }
}

#[tokio::test]
async fn orchestration_revision_loop_with_fake_planner() {
    // Create invalid spec (will fail compilation)
    let invalid_spec = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "invalid_field".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    // Create valid spec (will pass compilation)
    let valid_spec = ReportSpec {
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

    let planner = FakePlannerWithRevision::new(invalid_spec, valid_spec);
    let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation);

    // Load test registry
    let registry = load_test_registry();

    let ctx = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string()],
        vec!["campaigns_latest".to_string()],
    );

    let result = orchestrator
        .suggest_and_compile(&registry, "show me campaigns", ctx)
        .await;

    // Should succeed after revision
    match result {
        OrchestrationResult::Success {
            plan,
            draft,
            diffs: _,
            trace,
        } => {
            assert!(draft.is_some());
            assert_eq!(plan.workspace, "campaigns_offers");
            // Trace should show that a revision occurred
            if let Some(t) = trace {
                assert!(t.revisions_occurred);
                assert!(t.attempts > 1);
            }
        }
        other => panic!("Expected success after revision, got: {:?}", other),
    }
}

#[tokio::test]
async fn orchestration_max_attempts_exceeded() {
    // Create spec that will always fail compilation
    let invalid_spec = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "field_that_does_not_exist".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    // Both first and revision return same invalid spec
    let planner = FakePlannerWithRevision::new(invalid_spec.clone(), invalid_spec);
    let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation);

    // Load test registry
    let registry = load_test_registry();

    let ctx = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string()],
        vec!["campaigns_latest".to_string()],
    );

    let result = orchestrator
        .suggest_and_compile(&registry, "show me campaigns", ctx)
        .await;

    // Should fail after max attempts
    match result {
        OrchestrationResult::RetryLimitExceeded {
            diagnostics,
            draft: _,
            attempts,
        } => {
            assert!(!diagnostics.errors.is_empty());
            assert_eq!(attempts, 3); // Default max attempts
        }
        other => panic!("Expected retry limit exceeded, got: {:?}", other),
    }
}

#[tokio::test]
async fn orchestration_compile_only_path() {
    // Test compile-only flow (no planner involved)
    let valid_spec = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![Filter {
            field: "deleted".to_string(),
            op: FilterOp::Eq,
            value: serde_json::Value::Bool(false),
        }],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    // Load test registry
    let registry = load_test_registry();

    let orchestrator = Orchestrator::new(FixturePlanner::new(), AutoApproveConfirmation);
    let result = orchestrator.compile_only(&registry, &valid_spec);

    // Should succeed
    match result {
        OrchestrationResult::Success {
            plan,
            draft,
            diffs,
            trace,
        } => {
            assert!(draft.is_none()); // No draft in compile-only mode
            assert!(diffs.is_empty()); // No diffs in compile-only mode
            assert!(trace.is_none()); // No trace in compile-only mode
            assert_eq!(plan.workspace, "campaigns_offers");
        }
        other => panic!("Expected success, got: {:?}", other),
    }
}

#[tokio::test]
async fn orchestration_trace_captures_flow() {
    let valid_spec = ReportSpec {
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

    let mut planner = FixturePlanner::new();
    planner.add_fixture("test prompt".to_string(), valid_spec);

    let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation);

    let registry = load_test_registry();

    let ctx = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string()],
        vec!["campaigns_latest".to_string()],
    );

    let result = orchestrator
        .suggest_and_compile(&registry, "test prompt", ctx)
        .await;

    match result {
        OrchestrationResult::Success { trace, .. } => {
            assert!(trace.is_some());
            let t = trace.unwrap();
            // FixturePlanner uses model from context, which is "unknown" by default
            assert_eq!(t.attempts, 1);
            assert!(!t.revisions_occurred);
        }
        other => panic!("Expected success, got: {:?}", other),
    }
}
