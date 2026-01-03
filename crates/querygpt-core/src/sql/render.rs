use crate::dsl::plan::{IntermediatePlan, JoinType};
use anyhow::Result;

fn render_sql_inner(plan: &IntermediatePlan) -> String {
    let select_clause = if plan.projections.is_empty() {
        "SELECT 1".to_string()
    } else {
        let cols = plan.projections
            .iter()
            .map(|p| {
                if let Some(alias) = &p.alias {
                    format!("{} AS {}", p.expression, alias)
                } else {
                    p.expression.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",\n       ");

        format!("SELECT {}", cols)
    };

    // FROM (deterministic)
    let mut tables = plan.tables.clone();
    tables.sort_by(|a, b| a.alias.cmp(&b.alias));

    let from_clause = format!(
        "FROM {} {}",
        tables[0].name,
        tables[0].alias
    );

    // JOINs (deterministic)
    let mut joins = plan.joins.clone();
    joins.sort_by(|a, b| {
        (a.left_alias.clone(), a.right_alias.clone())
            .cmp(&(b.left_alias.clone(), b.right_alias.clone()))
    });

    let join_sql = joins
        .iter()
        .map(|j| {
            let join_type = match j.join_type {
                JoinType::Inner => "JOIN",
                JoinType::Left => "LEFT JOIN",
            };

            let on_clause = j.conditions
                .iter()
                .map(|c| format!("{} = {}", c.left_field, c.right_field))
                .collect::<Vec<_>>()
                .join(" AND ");

            format!(
                "{} {} {} ON {}",
                join_type,
                // lookup table name by alias
                plan.tables
                    .iter()
                    .find(|t| t.alias == j.right_alias)
                    .unwrap()
                    .name,
                j.right_alias,
                on_clause
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // WHERE
    let where_clause = if plan.filters.is_empty() {
        "".to_string()
    } else {
        let predicates = plan.filters
            .iter()
            .map(|f| f.expression.clone())
            .collect::<Vec<_>>()
            .join("\n  AND ");

        format!("\nWHERE {}", predicates)
    };

    format!(
        "{select}\n{from}\n{joins}{where}",
        select = select_clause,
        from = from_clause,
        joins = if join_sql.is_empty() { "".into() } else { format!("\n{}", join_sql) },
        where = where_clause,
    )
}


/// Stub: Render an intermediate plan into SQL, optionally with an LLM filling in details.
pub fn render_sql(plan: &IntermediatePlan) -> Result<String> {
    Ok(render_sql_inner(plan))
}
