use querygpt_core::dsl::report_spec::{Mode, ReportSpec, SelectItem};
use querygpt_core::planner::{
    planner::PlannerContext,
    prompt_templates::PromptTemplates,
    schema_summary::{ExamplePair, FieldSummary, PlannerConstraints, SchemaSummary, TableSummary},
};

#[test]
fn schema_summary_minimal_creates_basic_structure() {
    let workspace = "test_workspace";
    let summary = SchemaSummary::minimal(workspace);

    assert_eq!(summary.tables.len(), 1);
    assert_eq!(summary.tables[0].name, format!("{}_latest", workspace));
    assert_eq!(summary.tables[0].alias, workspace);
    assert_eq!(summary.tables[0].fields.len(), 1);
    assert_eq!(summary.tables[0].fields[0].name, "id");
    assert_eq!(summary.tables[0].fields[0].field_type, "uuid");
}

#[test]
fn schema_summary_get_all_fields_works() {
    let mut summary = SchemaSummary::minimal("test");

    // Add another table with different fields
    summary.tables.push(TableSummary {
        name: "offers".to_string(),
        alias: "o".to_string(),
        description: Some("Offers table".to_string()),
        fields: vec![
            FieldSummary {
                name: "offer_id".to_string(),
                field_type: "String".to_string(),
                nullable: false,
                description: Some("Offer identifier".to_string()),
                enum_values: None,
            },
            FieldSummary {
                name: "region".to_string(),
                field_type: "String".to_string(),
                nullable: true,
                description: Some("Geographic region".to_string()),
                enum_values: Some(vec!["APAC".to_string(), "EMEA".to_string()]),
            },
        ],
    });

    let all_fields = summary.get_all_fields();
    assert!(all_fields.contains(&"id".to_string()));
    assert!(all_fields.contains(&"offer_id".to_string()));
    assert!(all_fields.contains(&"region".to_string()));
    assert_eq!(all_fields.len(), 3);
}

#[test]
fn enhanced_planner_context_creates_rich_context() {
    let mut summary = SchemaSummary::minimal("campaigns_offers");

    // Add a more realistic table structure
    summary.tables[0].fields.push(FieldSummary {
        name: "campaign_id".to_string(),
        field_type: "String".to_string(),
        nullable: false,
        description: Some("Campaign identifier".to_string()),
        enum_values: None,
    });

    let examples = vec![ExamplePair {
        prompt: "Show me all campaigns".to_string(),
        spec: ReportSpec {
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
        },
        description: "Basic campaign listing".to_string(),
    }];

    let constraints = PlannerConstraints {
        max_select_fields: 10,
        max_filters: 5,
        allowed_workspaces: vec!["campaigns_offers".to_string()],
        forbidden_patterns: vec!["DROP".to_string(), "DELETE".to_string()],
    };

    let context = PlannerContext::enhanced(
        "campaigns_offers".to_string(),
        summary,
        examples,
        Some(constraints),
    );

    assert_eq!(context.workspace, "campaigns_offers");
    assert_eq!(context.examples.len(), 1);
    assert_eq!(context.constraints.max_select_fields, 10);
    assert!(context.available_fields.contains(&"id".to_string()));
    assert!(context
        .available_fields
        .contains(&"campaign_id".to_string()));
}

#[test]
fn enhanced_prompt_includes_schema_details() {
    let mut summary = SchemaSummary::minimal("test");
    summary.tables[0].description = Some("Test table for campaigns".to_string());
    summary.tables[0].fields[0].description = Some("Primary identifier".to_string());

    let examples = vec![ExamplePair {
        prompt: "test example".to_string(),
        spec: ReportSpec {
            version: 1,
            workspace: "test".to_string(),
            select: vec![SelectItem {
                field: "id".to_string(),
                alias: None,
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        },
        description: "Example description".to_string(),
    }];

    let context = PlannerContext::enhanced("test".to_string(), summary, examples, None);

    let prompt = PromptTemplates::system_prompt(&context);

    assert!(prompt.contains("Test table for campaigns"));
    assert!(prompt.contains("Primary identifier"));
    assert!(prompt.contains("EXAMPLES:"));
    assert!(prompt.contains("test example"));
    assert!(prompt.contains("Example description"));
}

#[test]
fn planner_constraints_default_values() {
    let constraints = PlannerConstraints::default();

    assert_eq!(constraints.max_select_fields, 20);
    assert_eq!(constraints.max_filters, 10);
    assert!(constraints.allowed_workspaces.is_empty());
    assert!(constraints.forbidden_patterns.is_empty());
}
