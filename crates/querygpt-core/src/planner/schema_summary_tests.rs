use crate::planner::{
    schema_summary::{SchemaSummary, TableSummary, FieldSummary, ExamplePair, PlannerConstraints},
    planner::PlannerContext,
    prompt_templates::PromptTemplates,
};
use crate::dsl::report_spec::{ReportSpec, SelectItem, Mode};

#[test]
fn schema_summary_minimal_creates_basic_structure() {
    let schema = SchemaSummary::minimal("campaigns");
    
    assert_eq!(schema.tables.len(), 1);
    assert_eq!(schema.tables[0].name, "campaigns_latest");
    assert_eq!(schema.tables[0].alias, "campaigns");
    assert_eq!(schema.tables[0].fields.len(), 1);
    assert_eq!(schema.tables[0].fields[0].name, "id");
}

#[test]
fn schema_summary_get_all_fields_works() {
    let schema = SchemaSummary {
        tables: vec![
            TableSummary {
                name: "offers".to_string(),
                alias: "o".to_string(),
                fields: vec![
                    FieldSummary {
                        name: "offer_id".to_string(),
                        field_type: "uuid".to_string(),
                        nullable: false,
                        description: None,
                        enum_values: None,
                    },
                    FieldSummary {
                        name: "region".to_string(),
                        field_type: "text".to_string(),
                        nullable: true,
                        description: None,
                        enum_values: Some(vec!["APAC".to_string(), "EMEA".to_string()]),
                    },
                ],
                description: None,
            },
        ],
        relationships: vec![],
        enums: vec![],
    };
    
    let fields = schema.get_all_fields();
    assert_eq!(fields, vec!["offer_id", "region"]);
}

#[test]
fn enhanced_planner_context_creates_rich_context() {
    let schema = SchemaSummary::minimal("test");
    let examples = vec![ExamplePair {
        prompt: "Show all items".to_string(),
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
        description: "Basic example".to_string(),
    }];
    
    let ctx = PlannerContext::enhanced(
        "test".to_string(),
        schema,
        examples,
        None,
    );
    
    assert_eq!(ctx.workspace, "test");
    assert_eq!(ctx.examples.len(), 1);
    assert_eq!(ctx.available_fields, vec!["id"]);
    assert_eq!(ctx.available_tables, vec!["test_latest"]);
}

#[test]
fn enhanced_prompt_includes_schema_details() {
    let schema = SchemaSummary {
        tables: vec![
            TableSummary {
                name: "offers".to_string(),
                alias: "o".to_string(),
                fields: vec![
                    FieldSummary {
                        name: "offer_id".to_string(),
                        field_type: "uuid".to_string(),
                        nullable: false,
                        description: Some("Primary key".to_string()),
                        enum_values: None,
                    },
                ],
                description: Some("Offers table".to_string()),
            },
        ],
        relationships: vec![],
        enums: vec![],
    };
    
    let ctx = PlannerContext::enhanced(
        "test".to_string(),
        schema,
        vec![],
        None,
    );
    
    let prompt = PromptTemplates::system_prompt(&ctx);
    
    assert!(prompt.contains("SCHEMA SUMMARY:"));
    assert!(prompt.contains("Table: offers (alias: o)"));
    assert!(prompt.contains("Description: Offers table"));
    assert!(prompt.contains("- offer_id (uuid, required)"));
    assert!(prompt.contains("Description: Primary key"));
}

#[test]
fn planner_constraints_default_values() {
    let constraints = PlannerConstraints::default();
    
    assert_eq!(constraints.max_select_fields, 20);
    assert_eq!(constraints.max_filters, 10);
    assert!(constraints.allowed_workspaces.is_empty());
    assert!(constraints.forbidden_patterns.is_empty());
}