use querygpt_core::dsl::plan::{IntermediatePlan, JoinCondition, JoinType, PlanJoin, PlanProjection, PlanTable};
use querygpt_core::sql::render::render_sql;

#[test]
fn group_by_added_when_aggregate_present() {
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
            PlanProjection { field: "products_csv".into(), expression: "STRING_AGG(DISTINCT opr.product_id, ',')".into(), alias: None },
        ],
        filters: vec![],
        order_by: vec![],
    };

    let sql = render_sql(&plan).unwrap();
    insta::assert_snapshot!(sql);
}
