use crate::dsl::plan::{IntermediatePlan, JoinCondition, JoinType, PlanJoin, PlanTable};
use crate::dsl::report_spec::ReportSpec;
use crate::schema::cards::SchemaCards;
use crate::schema::registry::SchemaRegistry;
use std::collections::HashMap;

use crate::compile::diagnostics::{CompileError, CompilerDiagnostics, CompilerError};
use crate::dsl::plan::PlanFilter;
use crate::dsl::report_spec::{Filter, FilterOp};
use anyhow::Result;
use serde_json::Value;

use crate::dsl::plan::PlanProjection;
use crate::dsl::report_spec::SelectItem;

use crate::dsl::plan::{PlanOrder, SortDirection};
use crate::dsl::report_spec::{OrderBy, SortDir};

type CompilerResult<T> = std::result::Result<T, CompilerError>;
type PaginationResult<T> = std::result::Result<T, CompileError>;

fn compile_pagination(spec: &ReportSpec) -> PaginationResult<(Option<u64>, Option<u64>)> {
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

/// Convert snake_case to camelCase for JSONB field matching
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn normalize_join_condition_for_aliases(
    left_alias: &str,
    right_alias: &str,
    mut c: JoinCondition,
) -> CompilerResult<JoinCondition> {
    let a = field_alias(&c.left_field);
    let b = field_alias(&c.right_field);

    match (a, b) {
        (Some(a1), Some(b1)) if a1 == left_alias && b1 == right_alias => Ok(c),
        (Some(a1), Some(b1)) if a1 == right_alias && b1 == left_alias => {
            std::mem::swap(&mut c.left_field, &mut c.right_field);
            Ok(c)
        }
        _ => Err(CompilerError::InvalidJoin {
            reason: format!(
                "cannot normalize join condition: {} = {} for join {} -> {}",
                c.left_field, c.right_field, left_alias, right_alias
            ),
        }),
    }
}

/// Translate a single field name into its SQL expression.
/// Uses schema cards to determine if field is a JSONB path or direct column.
/// Priority: 1) Semantic mappings, 2) JSONB paths, 3) Direct columns
fn field_to_sql_expr(
    field: &str,
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> Option<String> {
    // Priority 1: Semantic mappings (explicit user-friendly names - highest priority)
    // These are well-known field names that map to specific columns
    let semantic_result = match field {
        "partnership_id" => alias_map.get("partners").map(|a| format!("{}.id", a)),
        "campaign_id" => alias_map
            .get("campaigns_latest")
            .map(|a| format!("{}.id", a)),
        "campaign_name" => alias_map
            .get("campaigns_latest")
            .map(|a| format!("{}.name", a)),
        "offer_id" => alias_map.get("offers_latest").map(|a| format!("{}.id", a)),
        "offer_name" => alias_map
            .get("offers_latest")
            .map(|a| format!("{}.name", a)),
        "workflow_status" => alias_map
            .get("offers_latest")
            .map(|a| format!("{}.status", a)),
        "promo_type" => Some("promo_type".to_string()), // Special: resolved in filter translation
        _ => None,
    };
    if semantic_result.is_some() {
        return semantic_result;
    }

    // Priority 2: Check json_paths in schema (source of truth for JSONB attributes)
    // Try both exact match and camelCase conversion (e.g., package_id → packageId)
    let field_camel = to_camel_case(field);
    for entity in &cards.entities {
        if let Some(alias) = alias_map.get(&entity.name) {
            for json_path in &entity.json_paths {
                // Extract field name from $.fieldName
                let path_field = json_path.path.trim_start_matches("$.");
                if path_field == field || path_field == field_camel {
                    // Generate JSONB extraction SQL
                    return Some(format!(
                        "{}.{} ->> '{}'",
                        alias, json_path.column, path_field
                    ));
                }
            }
        }
    }

    // Priority 3: Check direct columns (skip promoted columns: start_date, end_date, status, countries)
    let skip_direct_columns = ["start_date", "end_date", "status", "countries"];
    if !skip_direct_columns.contains(&field) {
        for entity in &cards.entities {
            if let Some(alias) = alias_map.get(&entity.name) {
                if entity.columns.iter().any(|c| c.name == field) {
                    return Some(format!("{}.{}", alias, field));
                }
            }
        }
    }

    // Field not found - return None to trigger error
    None
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
) -> CompilerResult<Vec<PlanOrder>> {
    order_by
        .iter()
        .map(|item| {
            // Determine the SQL expression for ordering. Derived fields are handled
            // via replacement on the derived SQL (as in projections).
            let expr =
                if let Some(df) = cards.derived_fields.iter().find(|df| df.name == item.field) {
                    // Replace table names in the derived SQL with aliases
                    alias_map
                        .iter()
                        .fold(df.sql.clone(), |acc, (entity, alias)| {
                            acc.replace(&format!("{}.", entity), &format!("{}.", alias))
                        })
                } else {
                    // For direct fields, map to alias.column or fallback via field_to_sql_expr
                    field_to_sql_expr(&item.field, alias_map, cards).ok_or_else(|| {
                        CompilerError::UnknownField {
                            field: item.field.clone(),
                            context: "order_by",
                        }
                    })?
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
) -> CompilerResult<Vec<PlanProjection>> {
    select
        .iter()
        .map(|item| {
            // Determine the SQL expression for this field.
            let expr =
                if let Some(df) = cards.derived_fields.iter().find(|df| df.name == item.field) {
                    // Handle derived fields (expired_or_live_status, products_csv, etc.)
                    // Replace table names in the derived SQL with aliases
                    alias_map
                        .iter()
                        .fold(df.sql.clone(), |acc, (entity, alias)| {
                            acc.replace(&format!("{}.", entity), &format!("{}.", alias))
                        })
                } else {
                    // Use centralized field resolution (handles JSONB paths, direct columns, semantic mappings)
                    field_to_sql_expr(&item.field, alias_map, cards).ok_or_else(|| {
                        CompilerError::UnknownField {
                            field: item.field.clone(),
                            context: "select",
                        }
                    })?
                };

            // Auto-generate alias for JSON extractions to prevent ?column? in PostgreSQL
            let alias = if item.alias.is_none() && (expr.contains("->") || expr.contains("->>")) {
                Some(item.field.clone())
            } else {
                item.alias.clone()
            };

            Ok(PlanProjection {
                field: item.field.clone(),
                expression: expr,
                alias,
            })
        })
        .collect()
}

/// Translate a single filter into SQL.
/// Returns an error if the filter cannot be expressed.
fn translate_filter(
    filter: &Filter,
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> CompilerResult<String> {
    // First determine the SQL expression for the field.
    let column_sql = field_to_sql_expr(&filter.field, alias_map, cards).ok_or_else(|| {
        CompilerError::UnknownField {
            field: filter.field.clone(),
            context: "filters",
        }
    })?;

    match filter.op {
        FilterOp::Eq => {
            // Handle null checks with IS NULL
            if filter.value.is_null() {
                return Ok(format!("{} IS NULL", column_sql));
            }

            // Expect scalar values; wrap strings in single quotes.
            let rhs = match &filter.value {
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                _ => {
                    return Err(CompilerError::InvalidFilter {
                        field: filter.field.clone(),
                    })
                }
            };
            Ok(format!("{} = {}", column_sql, rhs))
        }
        FilterOp::In => {
            // Expect array of scalars; wrap string elements in quotes.
            let arr = match &filter.value {
                Value::Array(vals) if !vals.is_empty() => vals,
                _ => {
                    return Err(CompilerError::InvalidFilter {
                        field: filter.field.clone(),
                    })
                }
            };
            let vals_sql = arr
                .iter()
                .filter_map(|v| {
                    Some(match v {
                        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        _ => {
                            return None;
                        }
                    })
                })
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!("{} IN ({})", column_sql, vals_sql))
        }
        FilterOp::Overlaps => {
            // For array overlap queries (e.g. countries).
            let arr = match &filter.value {
                Value::Array(vals) if !vals.is_empty() => vals,
                _ => {
                    return Err(CompilerError::InvalidFilter {
                        field: filter.field.clone(),
                    })
                }
            };
            let elements_sql = arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", ");

            // Check if this is a JSONB field (indicated by ->> operator in column_sql)
            if column_sql.contains("->>") {
                // JSONB field: use ?| operator (JSONB contains any of)
                // Replace ->> with -> to get JSONB type instead of text
                let jsonb_column = column_sql.replace("->>", "->");
                // Wrap in parentheses to avoid operator precedence issues
                Ok(format!("({} ?| ARRAY[{}])", jsonb_column, elements_sql))
            } else {
                // Direct array column: use && operator
                Ok(format!("{} && ARRAY[{}]", column_sql, elements_sql))
            }
        }
        FilterOp::Gte | FilterOp::Lte => {
            // Greater-than or less-than comparisons (dates or numbers)
            let op_str = if matches!(filter.op, FilterOp::Gte) {
                ">="
            } else {
                "<="
            };
            let rhs = match &filter.value {
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Number(n) => n.to_string(),
                _ => {
                    return Err(CompilerError::InvalidFilter {
                        field: filter.field.clone(),
                    })
                }
            };
            Ok(format!("{} {} {}", column_sql, op_str, rhs))
        }
    }
}

/// Translate all filters of a report spec into a Vec<PlanFilter> (functional style).
pub fn translate_filters(
    filters: &[Filter],
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> CompilerResult<Vec<PlanFilter>> {
    // Use cards to resolve field names to SQL expressions (JSONB paths or direct columns)
    filters
        .iter()
        .map(|f| translate_filter(f, alias_map, cards).map(|sql| PlanFilter { expression: sql }))
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

    // 2b. Check if field exists in any entity's json_paths (JSONB fields)
    // Try both exact match and camelCase conversion (e.g., package_id → packageId)
    let field_camel = to_camel_case(field);
    for entity in &cards.entities {
        for json_path in &entity.json_paths {
            // Extract field name from $.fieldName
            let path_field = json_path.path.trim_start_matches("$.");
            if path_field == field || path_field == field_camel {
                return Some(entity.name.as_str());
            }
        }
    }

    // 2c. Otherwise scan all entities to see if the field matches a direct column name.
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
) -> CompilerResult<Vec<PlanJoin>> {
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
            required_names.contains(&edge.from.as_str())
                && required_names.contains(&edge.to.as_str())
        })
        .map(|edge| -> CompilerResult<PlanJoin> {
            let left_alias = alias_map
                .get(&edge.from)
                .ok_or_else(|| CompilerError::InvalidJoin {
                    reason: format!("missing alias_map entry for join edge.from '{}'", edge.from),
                })?
                .clone();

            let right_alias = alias_map
                .get(&edge.to)
                .ok_or_else(|| CompilerError::InvalidJoin {
                    reason: format!("missing alias_map entry for join edge.to '{}'", edge.to),
                })?
                .clone();

            let conditions = edge
                .on
                .iter()
                .map(|expr| -> CompilerResult<JoinCondition> {
                    let (left, right) =
                        expr.split_once('=')
                            .ok_or_else(|| CompilerError::InvalidJoin {
                                reason: format!("invalid join expression (missing '='): '{expr}'"),
                            })?;

                    let (left_tbl, left_col) =
                        left.trim()
                            .split_once('.')
                            .ok_or_else(|| CompilerError::InvalidJoin {
                                reason: format!(
                                    "invalid join LHS (expected tbl.col): '{}'",
                                    left.trim()
                                ),
                            })?;

                    let (right_tbl, right_col) =
                        right
                            .trim()
                            .split_once('.')
                            .ok_or_else(|| CompilerError::InvalidJoin {
                                reason: format!(
                                    "invalid join RHS (expected tbl.col): '{}'",
                                    right.trim()
                                ),
                            })?;

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
                .collect::<CompilerResult<Vec<_>>>()?;

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
        .collect::<CompilerResult<Vec<_>>>()
}

/// Stub: compile DSL into an intermediate plan (tables, joins, selected fields, predicates).
/// In production, this becomes the deterministic backbone that the LLM must follow.
pub fn compile_report_spec(
    reg: &SchemaRegistry,
    spec: &ReportSpec,
) -> Result<IntermediatePlan, CompilerDiagnostics> {
    compile_report_spec_internal(reg, spec).map_err(CompilerDiagnostics::from)
}

fn compile_report_spec_internal(
    reg: &SchemaRegistry,
    spec: &ReportSpec,
) -> CompilerResult<IntermediatePlan> {
    if reg.index.workspace != spec.workspace {
        return Err(CompilerError::SchemaMismatch {
            expected: reg.index.workspace.clone(),
            found: spec.workspace.clone(),
        });
    }

    let schema_cards = &reg.cards;

    let select_entities = spec
        .select
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards))
        .collect::<Vec<_>>();
    let filter_entities = spec
        .filters
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards))
        .collect::<Vec<_>>();
    let order_by_entities = spec
        .order_by
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards))
        .collect::<Vec<_>>();

    let required_entities = select_entities
        .iter()
        .chain(filter_entities.iter())
        .chain(order_by_entities.iter())
        .collect::<Vec<_>>();
    let tables = required_entities
        .iter()
        .filter_map(|e| {
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
                    alias: alias.to_string(),
                }
            })
        })
        .collect::<Vec<_>>();
    let alias_map: HashMap<String, String> = tables
        .iter()
        .map(|t| (t.name.clone(), t.alias.clone()))
        .collect();
    let joins = build_joins(&reg.cards, required_entities, &alias_map)?;
    let projections = translate_projections(&spec.select, &alias_map, &reg.cards)?;
    let filters = translate_filters(&spec.filters, &alias_map, &reg.cards)?;
    let order_by = translate_ordering(&spec.order_by, &alias_map, &reg.cards)?;
    let (limit, offset) = compile_pagination(spec)?;
    let plan = IntermediatePlan {
        workspace: spec.workspace.clone(),
        tables,
        joins,
        projections,
        filters,
        order_by,
        limit,
        offset,
    };
    Ok(plan)
}
