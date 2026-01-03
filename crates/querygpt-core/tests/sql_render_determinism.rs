use querygpt_core::dsl::plan::{IntermediatePlan, PlanTable, PlanJoin, JoinCondition, JoinType};
use querygpt_core::sql::render::render_sql;

#[test]
fn renderer_is_deterministic_given_same_plan() {
    // Same semantics, but tables appear in different order
    let plan_a = IntermediatePlan {
        workspace: "campaigns_offers".to_string(),
        tables: vec![
            PlanTable { name: "offers_latest".into(), alias: "o".into() },
            PlanTable { name: "campaign_offers".into(), alias: "co".into() },
        ],
        joins: vec![
            PlanJoin {
                left_alias: "o".into(),
                right_alias: "co".into(),
                join_type: JoinType::Inner,
                conditions: vec![
                    JoinCondition { left_field: "o.id".into(), right_field: "co.offer_id".into() },
                    JoinCondition { left_field: "o.profile".into(), right_field: "co.profile".into() },
                ],
            }
        ],
        projections: vec![],
        filters: vec![],
        order_by: vec![],
        limit: None,
        offset: None,
    };

    let plan_b = IntermediatePlan {
        tables: vec![
            PlanTable { name: "campaign_offers".into(), alias: "co".into() },
            PlanTable { name: "offers_latest".into(), alias: "o".into() },
        ],
        ..plan_a.clone()
    };

    let sql_a = render_sql(&plan_a).expect("render A");
    let sql_b = render_sql(&plan_b).expect("render B");

    assert_eq!(sql_a, sql_b, "SQL should be identical for semantically identical plans");
}
