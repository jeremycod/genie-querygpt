use querygpt_core::dsl::plan::*;
use querygpt_core::sql::render::render_sql;

#[test]
fn order_by_renders_in_plan_order() {
    let plan = IntermediatePlan {
        workspace: "campaigns_offers".into(),
        tables: vec![
            PlanTable { name: "offers_latest".into(), alias: "o".into() },
        ],
        joins: vec![],
        projections: vec![
            PlanProjection { field: "offer_id".into(), expression: "o.id".into(), alias: None },
        ],
        filters: vec![],
        order_by: vec![
            PlanOrder { expression: "o.id".into(), direction: SortDirection::Asc },
            PlanOrder { expression: "o.name".into(), direction: SortDirection::Desc },
        ],
        limit: None,
        offset: None,
    };

    let sql = render_sql(&plan).unwrap();
    insta::assert_snapshot!(sql);
}
