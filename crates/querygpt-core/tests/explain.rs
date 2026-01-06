use querygpt_core::dsl::plan::{
    IntermediatePlan, JoinCondition, JoinType, PlanFilter, PlanJoin, PlanProjection, PlanTable,
};
use querygpt_core::explain::explain::{
    explain_filters, explain_joins, explain_pagination, explain_plan,
};

fn sample_plan_with_relationships() -> IntermediatePlan {
    IntermediatePlan {
        workspace: "campaigns_offers".to_string(),
        tables: vec![
            PlanTable {
                name: "offers_latest".to_string(),
                alias: "o".to_string(),
            },
            PlanTable {
                name: "campaigns".to_string(),
                alias: "c".to_string(),
            },
            PlanTable {
                name: "partners".to_string(),
                alias: "p".to_string(),
            },
        ],
        joins: vec![
            PlanJoin {
                left_alias: "o".to_string(),
                right_alias: "c".to_string(),
                join_type: JoinType::Inner,
                conditions: vec![JoinCondition {
                    left_field: "o.id".to_string(),
                    right_field: "c.offer_id".to_string(),
                }],
            },
            PlanJoin {
                left_alias: "c".to_string(),
                right_alias: "p".to_string(),
                join_type: JoinType::Left,
                conditions: vec![JoinCondition {
                    left_field: "c.partner_id".to_string(),
                    right_field: "p.id".to_string(),
                }],
            },
        ],
        projections: vec![PlanProjection {
            field: "offer_id".to_string(),
            expression: "o.id".to_string(),
            alias: None,
        }],
        filters: vec![
            PlanFilter {
                expression: "o.status = 'PUBLISHED'".to_string(),
            },
            PlanFilter {
                expression: "p.active = TRUE".to_string(),
            },
        ],
        order_by: vec![],
        limit: Some(100),
        offset: Some(20),
    }
}

fn minimal_plan() -> IntermediatePlan {
    IntermediatePlan {
        workspace: "workspace_a".to_string(),
        tables: vec![PlanTable {
            name: "offers_latest".to_string(),
            alias: "o".to_string(),
        }],
        joins: vec![],
        projections: vec![],
        filters: vec![],
        order_by: vec![],
        limit: None,
        offset: None,
    }
}

#[test]
fn explains_all_sections_for_populated_plan() {
    let plan = sample_plan_with_relationships();

    let explanation = explain_plan(&plan);

    let expected = "\
Joins:
- INNER join offers_latest (o) to campaigns (c) on o.id = c.offer_id
- LEFT join campaigns (c) to partners (p) on c.partner_id = p.id

Filters:
- o.status = 'PUBLISHED'
- p.active = TRUE

Pagination:
- Limit 100 rows
- Offset 20 rows";

    assert_eq!(expected, explanation);
}

#[test]
fn handles_missing_sections_gracefully() {
    let plan = minimal_plan();

    assert_eq!("Joins:\n- No joins configured.", explain_joins(&plan));

    assert_eq!("Filters:\n- No filters applied.", explain_filters(&plan));

    assert_eq!(
        "Pagination:\n- No pagination (returns all rows).",
        explain_pagination(&plan)
    );
}
