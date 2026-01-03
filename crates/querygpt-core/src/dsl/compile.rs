use std::collections::HashMap;
use std::fmt;
use crate::dsl::plan::{IntermediatePlan, JoinCondition, JoinType, PlanJoin, PlanTable};
use crate::dsl::report_spec::ReportSpec;
use crate::schema::cards::SchemaCards;
use crate::schema::registry::SchemaRegistry;

use crate::dsl::plan::{PlanFilter};
use crate::dsl::report_spec::{Filter, FilterOp};
use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::dsl::plan::{PlanProjection};
use crate::dsl::report_spec::SelectItem;


use crate::dsl::plan::{PlanOrder, SortDirection};
use crate::dsl::report_spec::{OrderBy, SortDir};

#[derive(Debug)]
pub enum CompileError {
    InvalidLimit { value: i64 },
    InvalidOffset { value: i64 },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::InvalidLimit { value } => write!(f, "Invalid limit: {}", value),
            CompileError::InvalidOffset { value } => write!(f, "Invalid offset: {}", value),
        }
    }
}

impl std::error::Error for CompileError {}

fn compile_pagination(spec: &ReportSpec) -> Result<(Option<u64>, Option<u64>), CompileError> {
    let limit = spec.pagination.as_ref().and_then(|p| p.limit);
    let offset = spec.pagination.as_ref().and_then(|p| p.offset);

    // If your spec uses signed ints, validate >= 0 then cast.
    // If your spec already uses u64, most of this disappears.
    let limit_u = match limit {
        None => None,
        Some(v) if v >= 0 => Some(v as u64),
        Some(v) => return Err(CompileError::InvalidLimit { value: v }),
    };

    let offset_u = match offset {
        None => None,
        Some(v) if v >= 0 => Some(v as u64),
        Some(v) => return Err(CompileError::InvalidOffset { value: v }),
    };

    Ok((limit_u, offset_u))
}


fn field_alias(field: &str) -> Option<&str> {
    field.split('.').next()
}

fn normalize_join_condition_for_aliases(
    left_alias: &str,
    right_alias: &str,
    mut c: JoinCondition,
) -> Result<JoinCondition> {
    let a = field_alias(&c.left_field);
    let b = field_alias(&c.right_field);

    match (a, b) {
        (Some(a1), Some(b1)) if a1 == left_alias && b1 == right_alias => Ok(c),
        (Some(a1), Some(b1)) if a1 == right_alias && b1 == left_alias => {
            std::mem::swap(&mut c.left_field, &mut c.right_field);
            Ok(c)
        }
        _ => Err(anyhow!(
            "cannot normalize join condition: {} = {} for join {} -> {}",
            c.left_field,
            c.right_field,
            left_alias,
            right_alias
        )),
    }
}

/// Translate a single field name into its SQL expression, reusing the same logic as in projections.
/// Uses the alias_map to prefix columns and derives JSON paths where needed.
fn field_to_sql_expr(field: &str, alias_map: &HashMap<String, String>) -> Option<String> {
    Some(match field {
        // Direct fields on known entities
        "partnership_id" => format!("{}.id", alias_map.get("partners")?),
        "campaign_id" => format!("{}.id", alias_map.get("campaigns_latest")?),
        "campaign_name" => format!("{}.name", alias_map.get("campaigns_latest")?),
        "offer_id" => format!("{}.id", alias_map.get("offers_latest")?),
        "offer_name" => format!("{}.name", alias_map.get("offers_latest")?),
        "workflow_status" => format!("{}.status", alias_map.get("offers_latest")?),
        "countries" => format!("{}.countries", alias_map.get("offers_latest")?),
        "package_id" => format!("{}.attributes ->> 'packageId'", alias_map.get("offers_latest")?),
        other => other.to_string(), // fallback to raw field name
    })
}

/// Translate the order_by specifications into PlanOrder entries.
///
/// It uses the same field-to-expression mapping as in projections, then sets
/// SortDirection based on the `dir` (asc/desc). Returns an error if a field
/// cannot be mapped or an alias is missing.
pub fn translate_ordering(
    order_by: &[OrderBy],
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> Result<Vec<PlanOrder>> {
    order_by
        .iter()
        .map(|item| {
            // Determine the SQL expression for ordering. Derived fields are handled
            // via replacement on the derived SQL (as in projections).
            let expr = if let Some(df) = cards.derived_fields.iter().find(|df| df.name == item.field) {
                // Replace table names in the derived SQL with aliases
                alias_map
                    .iter()
                    .fold(df.sql.clone(), |acc, (entity, alias)| {
                        acc.replace(&format!("{}.", entity), &format!("{}.", alias))
                    })
            } else {
                // For direct fields, map to alias.column or fallback via field_to_sql_expr
                field_to_sql_expr(&item.field, alias_map)
                    .ok_or_else(|| anyhow!("cannot map order_by field {}", item.field))?
            };

            // Map direction to SortDirection
            let direction = match item.dir {
                SortDir::Asc => SortDirection::Asc,
                SortDir::Desc => SortDirection::Desc,
            };

            Ok(PlanOrder {
                expression: expr,
                direction,
            })
        })
        .collect()
}


/// Translate the select list into SQL projections.
/// Each entry becomes a PlanProjection containing:
///   - field: the original report field name
///   - expression: the SQL expression with table aliases
///   - alias: an optional alias provided in the ReportSpec
pub fn translate_projections(
    select: &[SelectItem],
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> Result<Vec<PlanProjection>> {
    select
        .iter()
        .map(|item| {
            // Determine the SQL expression for this field.
            let expr = match item.field.as_str() {
                // Direct fields that map to simple column names
                "partnership_id" => {
                    format!("{}.id", alias_map.get("partners").ok_or_else(|| {
                        anyhow!("missing alias for partners when rendering partnership_id")
                    })?)
                }
                "campaign_id" => {
                    format!("{}.id", alias_map.get("campaigns_latest").ok_or_else(|| {
                        anyhow!("missing alias for campaigns_latest when rendering campaign_id")
                    })?)
                }
                "campaign_name" => {
                    format!("{}.name", alias_map.get("campaigns_latest").ok_or_else(|| {
                        anyhow!("missing alias for campaigns_latest when rendering campaign_name")
                    })?)
                }
                "offer_id" => {
                    format!("{}.id", alias_map.get("offers_latest").ok_or_else(|| {
                        anyhow!("missing alias for offers_latest when rendering offer_id")
                    })?)
                }
                "offer_name" => {
                    format!("{}.name", alias_map.get("offers_latest").ok_or_else(|| {
                        anyhow!("missing alias for offers_latest when rendering offer_name")
                    })?)
                }
                "workflow_status" => {
                    // workflow_status is stored in offers_latest.status
                    format!("{}.status", alias_map.get("offers_latest").ok_or_else(|| {
                        anyhow!("missing alias for offers_latest when rendering workflow_status")
                    })?)
                }
                "countries" => {
                    format!("{}.countries", alias_map.get("offers_latest").ok_or_else(|| {
                        anyhow!("missing alias for offers_latest when rendering countries")
                    })?)
                }
                // Derived or special-case fields
                "package_id" => {
                    // Map to the JSON path attributes->>'packageId' on offers_latest
                    format!(
                        "{}.attributes ->> 'packageId'",
                        alias_map.get("offers_latest").ok_or_else(|| {
                            anyhow!("missing alias for offers_latest when rendering package_id")
                        })?
                    )
                }
                // Fields defined in derived_fields (expired_or_live_status, products_csv, etc.)
                other => {
                    if let Some(df) = cards.derived_fields.iter().find(|df| df.name == other) {
                        // Replace table names in the derived SQL with aliases
                        alias_map
                            .iter()
                            .fold(df.sql.clone(), |acc, (entity, alias)| {
                                acc.replace(&format!("{}.", entity), &format!("{}.", alias))
                            })
                    } else {
                        // Fallback: direct column with field name (for unknown but valid columns)
                        // Attempt to resolve via resolve_entity and then prefix alias
                        let entity = resolve_entity(other, cards).ok_or_else(|| {
                            anyhow!("cannot find entity for projection field {}", other)
                        })?;
                        let alias = alias_map.get(entity).ok_or_else(|| {
                            anyhow!("missing alias for {} when rendering {}", entity, other)
                        })?;
                        format!("{}.{}", alias, other)
                    }
                }
            };

            Ok(PlanProjection {
                field: item.field.clone(),
                expression: expr,
                alias: item.alias.clone(),
            })
        })
        .collect()
}




/// Translate a single filter into SQL.
/// Returns None if the filter cannot be expressed.
fn translate_filter(filter: &Filter, alias_map: &HashMap<String, String>) -> Option<String> {
    // First determine the SQL expression for the field.
    let column_sql = field_to_sql_expr(&filter.field, alias_map)?;

    match filter.op {
        FilterOp::Eq => {
            // Expect scalar values; wrap strings in single quotes.
            let rhs = match &filter.value {
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                _ => return None,
            };
            Some(format!("{} = {}", column_sql, rhs))
        }
        FilterOp::In => {
            // Expect array of scalars; wrap string elements in quotes.
            let arr = match &filter.value {
                Value::Array(vals) if !vals.is_empty() => vals,
                _ => return None,
            };
            let vals_sql = arr
                .iter()
                .filter_map(|v| {
                    Some(match v {
                        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        _ => return None?,
                    })
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("{} IN ({})", column_sql, vals_sql))
        }
        FilterOp::Overlaps => {
            // For array overlap queries (e.g. countries).
            let arr = match &filter.value {
                Value::Array(vals) if !vals.is_empty() => vals,
                _ => return None,
            };
            let elements_sql = arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("{} && ARRAY[{}]", column_sql, elements_sql))
        }
        FilterOp::Gte | FilterOp::Lte => {
            // Greater-than or less-than comparisons (dates or numbers)
            let op_str = if matches!(filter.op, FilterOp::Gte) { ">=" } else { "<=" };
            let rhs = match &filter.value {
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Number(n) => n.to_string(),
                _ => return None,
            };
            Some(format!("{} {} {}", column_sql, op_str, rhs))
        }
    }
}

/// Translate all filters of a report spec into a Vec<PlanFilter> (functional style).
pub fn translate_filters(
    filters: &[Filter],
    alias_map: &HashMap<String, String>,
    _cards: &SchemaCards,
) -> Result<Vec<PlanFilter>> {
    // The `cards` parameter is included for future extensions (e.g. dynamic derived field discovery),
    // but not used in this minimal example.
    filters
        .iter()
        .map(|f| {
            translate_filter(f, alias_map)
                .map(|sql| PlanFilter { expression: sql })
                .ok_or_else(|| anyhow!("invalid filter: {:?}", f))
        })
        .collect()
}


fn resolve_entity<'a>(field: &str, cards: &'a SchemaCards) -> Option<&'a str> {
    // 1. Hard-coded mapping for the campaigns_offers workspace
    match field {
        // partner-level field
        "partnership_id" => return Some("partners"),
        // campaign-level fields
        "campaign_id" | "campaign_name" => return Some("campaigns_latest"),
        // offer-level fields (direct columns)
        "offer_id" | "offer_name" | "workflow_status" | "countries" | "package_id" => {
            return Some("offers_latest")
        }
        // derived fields that live on offers_latest
        "expired_or_live_status" => return Some("offers_latest"),
        // derived aggregation that comes from offer_products
        "products_csv" => return Some("offer_products"),
        // filter-only field that comes from offer_phases
        "promo_type" => return Some("offer_phases"),
        _ => { /* fall through to dynamic lookup */ }
    }

    // 2. Dynamic lookup for other cases
    // 2a. If this is a derived field defined in schema_cards, inspect its dependencies.
    if let Some(derived) = cards.derived_fields.iter().find(|df| df.name == field) {
        // e.g. "offers_latest.end_date" ⇒ entity is "offers_latest"
        if let Some(dep) = derived.depends_on.first() {
            if let Some((entity, _)) = dep.split_once('.') {
                return Some(entity);
            }
        }
    }

    // 2b. Otherwise scan all entities to see if the field matches a direct column name.
    for entity in &cards.entities {
        if entity.columns.iter().any(|col| col.name == field) {
            return Some(entity.name.as_str());
        }
    }

    // Not found
    None
}
fn build_joins(
    cards: &SchemaCards,
    required: Vec<&Option<&str>>,
    alias_map: &HashMap<String, String>,
) -> Result<Vec<PlanJoin>> {
    let required_names: Vec<&str> = required
        .iter()
        .filter_map(|opt| opt.as_ref())
        .copied()
        .collect();

    cards
        .join_graph
        .edges
        .iter()
        .filter(|edge| {
            required_names.contains(&edge.from.as_str()) && required_names.contains(&edge.to.as_str())
        })
        .map(|edge| -> Result<PlanJoin> {
            let left_alias = alias_map
                .get(&edge.from)
                .ok_or_else(|| anyhow!("missing alias_map entry for join edge.from '{}'", edge.from))?
                .clone();

            let right_alias = alias_map
                .get(&edge.to)
                .ok_or_else(|| anyhow!("missing alias_map entry for join edge.to '{}'", edge.to))?
                .clone();

            let conditions = edge
                .on
                .iter()
                .map(|expr| -> Result<JoinCondition> {
                    let (left, right) = expr
                        .split_once('=')
                        .ok_or_else(|| anyhow!("invalid join expression (missing '='): '{}'", expr))?;

                    let (left_tbl, left_col) = left
                        .trim()
                        .split_once('.')
                        .ok_or_else(|| anyhow!("invalid join LHS (expected tbl.col): '{}'", left.trim()))?;

                    let (right_tbl, right_col) = right
                        .trim()
                        .split_once('.')
                        .ok_or_else(|| anyhow!("invalid join RHS (expected tbl.col): '{}'", right.trim()))?;

                    // Apply alias_map so the condition uses the plan aliases ("o.id", "oph.offer_id", etc.)
                    let left_prefix = alias_map
                        .get(left_tbl)
                        .cloned()
                        .unwrap_or_else(|| left_tbl.to_string());
                    let right_prefix = alias_map
                        .get(right_tbl)
                        .cloned()
                        .unwrap_or_else(|| right_tbl.to_string());

                    let c = JoinCondition {
                        left_field: format!("{}.{}", left_prefix, left_col),
                        right_field: format!("{}.{}", right_prefix, right_col),
                    };

                    // ✅ Normalize so condition is always (left_alias.*) = (right_alias.*)
                    normalize_join_condition_for_aliases(&left_alias, &right_alias, c)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(PlanJoin {
                left_alias,
                right_alias,
                join_type: match edge.join_type.as_str() {
                    "left" => JoinType::Left,
                    _ => JoinType::Inner,
                },
                conditions,
            })
        })
        .collect::<Result<Vec<_>>>()
}

/// Stub: compile DSL into an intermediate plan (tables, joins, selected fields, predicates).
/// In production, this becomes the deterministic backbone that the LLM must follow.
pub fn compile_report_spec(reg: &SchemaRegistry, spec: &ReportSpec) -> anyhow::Result<IntermediatePlan> {
    if reg.index.workspace != spec.workspace {
        return Err(anyhow::anyhow!(
            "workspace mismatch: expected {}, found {}",
            spec.workspace,
            reg.index.workspace
        ));
    }

    let schema_cards = &reg.cards;

    let select_entities = spec.select.iter().map(|s| resolve_entity(&s.field, schema_cards)).collect::<Vec<_>>();
    let filter_entities = spec.filters.iter().map(|s| resolve_entity(&s.field, schema_cards)).collect::<Vec<_>>();
    let order_by_entities = spec.order_by.iter().map(|s| resolve_entity(&s.field, schema_cards)).collect::<Vec<_>>();

    let required_entities = select_entities.iter().chain(filter_entities.iter()).chain(order_by_entities.iter()).collect::<Vec<_>>();
    let tables = required_entities.iter().filter_map(|e| {
        e.as_ref().map(|entity| {
            let alias = match *entity {
                "offers_latest" => "o",
                "campaigns_latest" => "c",
                "campaign_offers" => "co",
                "offer_products" => "opr",
                "offer_phases" => "oph",
                "partners" => "p",
                other => other,
            };
            PlanTable {
                name: entity.to_string(),
                alias: alias.to_string()
            }
        })
    }).collect::<Vec<_>>();
    let alias_map: HashMap<String, String> = tables.iter().map(|t| (t.name.clone(), t.alias.clone())).collect();
    let joins = build_joins(&reg.cards, required_entities, &alias_map)?;
    let projections = translate_projections(&spec.select, &alias_map, &reg.cards)?;
    let filters = translate_filters(&spec.filters, &alias_map, &reg.cards)?;
    let order_by = translate_ordering(&spec.order_by, &alias_map, &reg.cards)?;
    let (limit, offset) = compile_pagination(&spec)?;
    let plan = IntermediatePlan {
        workspace: spec.workspace.clone(),
        tables,
        joins,
        projections,
        filters,
        order_by,
        limit,
        offset
    };
    Ok(plan)
}
