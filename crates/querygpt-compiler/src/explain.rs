use std::collections::HashMap;

use querygpt_core::dsl::plan::{IntermediatePlan, JoinType, PlanJoin};

pub fn explain_joins(plan: &IntermediatePlan) -> String {
    let table_names = table_lookup(plan);

    if plan.joins.is_empty() {
        return "Joins:\n- No joins configured.".to_string();
    }

    let mut lines = vec!["Joins:".to_string()];
    lines.extend(
        plan.joins
            .iter()
            .map(|join| format!("- {}", describe_join(join, &table_names))),
    );

    lines.join("\n")
}

pub fn explain_filters(plan: &IntermediatePlan) -> String {
    if plan.filters.is_empty() {
        return "Filters:\n- No filters applied.".to_string();
    }

    let mut lines = vec!["Filters:".to_string()];
    lines.extend(
        plan.filters
            .iter()
            .map(|filter| format!("- {}", filter.expression)),
    );
    lines.join("\n")
}

pub fn explain_pagination(plan: &IntermediatePlan) -> String {
    match (plan.limit, plan.offset) {
        (None, None) => "Pagination:\n- No pagination (returns all rows).".to_string(),
        _ => {
            let mut lines = vec!["Pagination:".to_string()];

            match plan.limit {
                Some(limit) => lines.push(format!("- Limit {limit} rows")),
                None => lines.push("- No limit set".to_string()),
            }

            match plan.offset {
                Some(offset) => lines.push(format!("- Offset {offset} rows")),
                None => lines.push("- Offset not set".to_string()),
            }

            lines.join("\n")
        }
    }
}

pub fn explain_plan(plan: &IntermediatePlan) -> String {
    vec![
        explain_joins(plan),
        explain_filters(plan),
        explain_pagination(plan),
    ]
    .join("\n\n")
}

pub fn format_join_condition(join_condition: &querygpt_core::dsl::plan::JoinCondition) -> String {
    format!(
        "{} = {}",
        join_condition.left_field, join_condition.right_field
    )
}

fn describe_join(join: &PlanJoin, table_names: &HashMap<&str, &str>) -> String {
    let join_type = match join.join_type {
        JoinType::Inner => "INNER",
        JoinType::Left => "LEFT",
    };

    let left = describe_table_alias(&join.left_alias, table_names);
    let right = describe_table_alias(&join.right_alias, table_names);
    let conditions = if join.conditions.is_empty() {
        "no join condition".to_string()
    } else {
        join.conditions
            .iter()
            .map(format_join_condition)
            .collect::<Vec<_>>()
            .join(" AND ")
    };

    format!("{join_type} join {left} to {right} on {conditions}")
}

fn describe_table_alias<'a>(alias: &str, table_names: &'a HashMap<&str, &str>) -> String {
    if let Some(table_name) = table_names.get(alias) {
        format!("{table_name} ({alias})")
    } else {
        format!("alias {alias}")
    }
}

fn table_lookup(plan: &IntermediatePlan) -> HashMap<&str, &str> {
    plan.tables
        .iter()
        .map(|table| (table.alias.as_str(), table.name.as_str()))
        .collect()
}
