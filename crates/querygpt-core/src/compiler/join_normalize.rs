use anyhow::{anyhow, Result};
use crate::dsl::plan::{PlanJoin, JoinCondition};

fn field_alias(field: &str) -> Option<&str> {
    field.split('.').next()
}

pub fn normalize_plan_joins(joins: Vec<PlanJoin>) -> Result<Vec<PlanJoin>> {
    joins.into_iter().map(normalize_join).collect()
}


fn normalize_join(j: PlanJoin) -> Result<PlanJoin> {
    let left = j.left_alias.as_str();
    let right = j.right_alias.as_str();

    let norm_conditions: Result<Vec<JoinCondition>> = j
        .conditions
        .iter()
        .map(|c| {
            let a = field_alias(&c.left_field);
            let b = field_alias(&c.right_field);

            match (a, b) {
                (Some(a1), Some(b1)) if a1 == left && b1 == right => Ok(c.clone()),
                (Some(a1), Some(b1)) if a1 == right && b1 == left => Ok(JoinCondition {
                    left_field: c.right_field.clone(),
                    right_field: c.left_field.clone(),
                }),
                _ => Err(anyhow!(
                    "cannot normalize join condition: {} = {} for join {} -> {}",
                    c.left_field,
                    c.right_field,
                    left,
                    right
                )),
            }
        })
        .collect();

    Ok(PlanJoin {
        conditions: norm_conditions?,
        ..j
    })
}
