
use querygpt_core::dsl::compile::compile_report_spec;
use querygpt_core::dsl::plan::{IntermediatePlan, JoinCondition, JoinType, PlanJoin, PlanOrder, PlanProjection, PlanTable, SortDirection};
use querygpt_core::dsl::report_spec::ReportSpec;
use querygpt_core::schema::registry::SchemaRegistry;
use querygpt_core::sql::render::render_sql;

fn normalize_sql(s: &str) -> String {
    s.lines().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n").trim().to_string()
}

#[test]
fn full_query_snapshot_from_reportspec() {
    // Change to repo root directory for schema loading
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent().and_then(|p| p.parent())
        .expect("resolve repo root from CARGO_MANIFEST_DIR");
    
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&repo_root).expect("change to repo root");
    
    let reg = SchemaRegistry::load("config/workspaces/campaigns_offers.index.json")
        .expect("load SchemaRegistry");
    
    std::env::set_current_dir(original_dir).expect("restore original directory");

    let spec_str = include_str!("fixtures/report_specs/campaigns_offers_prepaid_apac.json");
    let spec: ReportSpec = serde_json::from_str(spec_str).expect("parse ReportSpec");

    let plan = compile_report_spec(&reg, &spec).expect("compile plan");
    let sql = render_sql(&plan).expect("render sql");

    insta::assert_snapshot!(normalize_sql(&sql));
}

#[test]
fn full_query_with_group_by_and_order_by() {
    let plan = IntermediatePlan {
        workspace: "campaigns_offers".into(),
        tables: vec![
            PlanTable { name: "offers_latest".into(), alias: "o".into() },
            PlanTable { name: "offer_products".into(), alias: "opr".into() },
        ],
        joins: vec![
            PlanJoin {
                left_alias: "o".into(),
                right_alias: "opr".into(),
                join_type: JoinType::Inner,
                conditions: vec![
                    JoinCondition { left_field: "o.id".into(), right_field: "opr.offer_id".into() },
                    JoinCondition { left_field: "o.profile".into(), right_field: "opr.profile".into() },
                    JoinCondition { left_field: "o.version".into(), right_field: "opr.version".into() },
                ],
            }
        ],
        projections: vec![
            PlanProjection { field: "offer_id".into(), expression: "o.id".into(), alias: None },
            PlanProjection {
                field: "products_csv".into(),
                expression: "STRING_AGG(DISTINCT opr.product_id, ',')".into(),
                alias: None,
            },
        ],
        filters: vec![],
        order_by: vec![
            PlanOrder { expression: "o.id".into(), direction: SortDirection::Asc },
        ],
        limit: None,
        offset: None,
    };

    let sql = render_sql(&plan).unwrap();
    insta::assert_snapshot!(sql);
}
