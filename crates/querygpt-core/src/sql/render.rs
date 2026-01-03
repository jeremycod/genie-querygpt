
use std::collections::BTreeSet;
use anyhow::{anyhow, Result};
use crate::dsl::plan::{IntermediatePlan, PlanJoin, JoinType, SortDirection, JoinCondition};



fn field_alias(field: &str) -> Option<&str> {
    field.split('.').next()
}

fn normalize_join(j: &PlanJoin) -> Result<PlanJoin> {
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
                    c.left_field, c.right_field, left, right
                )),
            }
        })
        .collect();

    Ok(PlanJoin {
        conditions: norm_conditions?,
        ..j.clone()
    })
}

fn render_order_by(plan: &IntermediatePlan) -> String {
    if plan.order_by.is_empty() {
        return String::new();
    }

    let items = plan
        .order_by
        .iter()
        .map(|o| {
            let dir = match o.direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            format!("{} {}", o.expression, dir)
        })
        .collect::<Vec<_>>()
        .join(",\n         ");

    format!("\nORDER BY {}", items)
}


fn is_aggregate_expr(expr: &str) -> bool {
    // Minimal heuristics good enough for v1:
    // You can extend later (COUNT, SUM, MIN, MAX, AVG, ARRAY_AGG, BOOL_AND, etc.)
    let upper = expr.to_ascii_uppercase();
    upper.contains("STRING_AGG(")
        || upper.contains("COUNT(")
        || upper.contains("SUM(")
        || upper.contains("MIN(")
        || upper.contains("MAX(")
        || upper.contains("AVG(")
        || upper.contains("ARRAY_AGG(")
}
fn group_by_exprs(plan: &IntermediatePlan) -> Vec<String> {
    let has_agg = plan.projections.iter().any(|p| is_aggregate_expr(&p.expression));

    if !has_agg {
        return vec![];
    }

    plan.projections
        .iter()
        .filter(|p| !is_aggregate_expr(&p.expression))
        .map(|p| p.expression.clone())
        .collect()
}
fn render_group_by(plan: &IntermediatePlan) -> String {
    let exprs = group_by_exprs(plan);
    if exprs.is_empty() {
        "".to_string()
    } else {
        format!("\nGROUP BY {}", exprs.join(",\n         "))
    }
}



fn choose_root_alias(plan: &IntermediatePlan) -> Result<String> {
    let table_aliases: BTreeSet<String> =
        plan.tables.iter().map(|t| t.alias.clone()).collect();

    let right_aliases: BTreeSet<String> =
        plan.joins.iter().map(|j| j.right_alias.clone()).collect();

    let left_aliases: BTreeSet<String> =
        plan.joins.iter().map(|j| j.left_alias.clone()).collect();
    
    // First try: tables that are never joined TO (traditional root selection)
    let traditional_roots: BTreeSet<String> = table_aliases.difference(&right_aliases).cloned().collect();
    
    // If we have joins, prefer a root that appears as a left_alias (can start a join chain)
    if !plan.joins.is_empty() {
        let viable_roots: BTreeSet<String> = traditional_roots.intersection(&left_aliases).cloned().collect();
        
        if let Some(root) = viable_roots.iter().next() {
            return Ok(root.clone());
        }
    }
    
    // Fallback to any traditional root
    traditional_roots
        .iter()
        .next()
        .cloned()
        .ok_or_else(|| anyhow!("cannot determine root alias: plan has no tables or join graph is cyclic"))
}

fn sorted_joins(mut joins: Vec<PlanJoin>) -> Vec<PlanJoin> {
    // (If you want absolutely no `mut` anywhere, see note below.)
    joins.sort_by(|a, b| {
        (a.left_alias.clone(), a.right_alias.clone())
            .cmp(&(b.left_alias.clone(), b.right_alias.clone()))
    });
    joins
}
/// Returns joins in a deterministic order such that each join's left_alias is already introduced.
fn order_joins(plan: &IntermediatePlan, root: &str) -> Result<Vec<PlanJoin>> {
    fn step(
        visited: BTreeSet<String>,
        remaining: Vec<PlanJoin>,
        acc: Vec<PlanJoin>,
    ) -> Result<Vec<PlanJoin>> {
        let (ready, not_ready): (Vec<PlanJoin>, Vec<PlanJoin>) = remaining
            .into_iter()
            .partition(|j| visited.contains(&j.left_alias) && !visited.contains(&j.right_alias));

        if ready.is_empty() {
            if not_ready.is_empty() {
                return Ok(acc);
            }
            
            // Check if we have disconnected components - find unvisited left aliases
            let unvisited_left_aliases: BTreeSet<String> = not_ready
                .iter()
                .map(|j| j.left_alias.clone())
                .filter(|alias| !visited.contains(alias))
                .collect();
                
            if !unvisited_left_aliases.is_empty() {
                // Add the first unvisited left alias to visited to continue processing
                let new_root = unvisited_left_aliases.iter().next().unwrap().clone();
                let mut new_visited = visited;
                new_visited.insert(new_root);
                return step(new_visited, not_ready, acc);
            }
            
            return Err(anyhow!(
                "cannot order joins: disconnected or wrong directions. remaining: {:?}",
                not_ready
            ));
        }

        let visited2: BTreeSet<String> = visited
            .into_iter()
            .chain(ready.iter().map(|j| j.right_alias.clone()))
            .collect();

        let acc2: Vec<PlanJoin> = acc
            .into_iter()
            .chain(ready.into_iter())
            .collect();

        step(visited2, not_ready, acc2)
    }

    let remaining = sorted_joins(
        plan.joins
            .iter()
            .map(normalize_join)
            .collect::<Result<Vec<_>>>()?
    );
    
    let visited: BTreeSet<String> = [root.to_string()].into_iter().collect();

    step(visited, remaining, Vec::new())
}



fn render_sql_inner(plan: &IntermediatePlan) -> Result<String> {
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

    // FROM (deterministic + valid)
    let root_alias = choose_root_alias(plan).unwrap();
    let root_table = plan.tables.iter().find(|t| t.alias == root_alias).unwrap();
    let from_clause = format!("FROM {} {}", root_table.name, root_table.alias);

    // JOINs (deterministic + valid)
    let join_sql = order_joins(plan, &root_alias)?
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

            let right_table_name = plan.tables
                .iter()
                .find(|t| t.alias == j.right_alias)
                .unwrap()
                .name
                .clone();

            format!(
                "{} {} {} ON {}",
                join_type,
                right_table_name,
                j.right_alias,
                on_clause
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

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
    let group_by_clause = render_group_by(plan);
    let order_by_clause = render_order_by(plan);
    let final_sql = format!(
        "{select}\n{from}\n{joins}{where}{group_by}{order_by}",
        select = select_clause,
        from = from_clause,
        joins = if join_sql.is_empty() { "".into() } else { format!("\n{}", join_sql) },
        where = where_clause,
        group_by = group_by_clause,
        order_by = order_by_clause
    );
    Ok(final_sql)
}



/// Stub: Render an intermediate plan into SQL, optionally with an LLM filling in details.
pub fn render_sql(plan: &IntermediatePlan) -> Result<String> {
    render_sql_inner(plan)
}
