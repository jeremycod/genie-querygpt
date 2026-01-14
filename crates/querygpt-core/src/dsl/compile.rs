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

/// Convert camelCase to snake_case for SQL column aliases
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            // Add underscore before uppercase letter, except at the start
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
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
        "campaign_startDate" => alias_map
            .get("campaigns_latest")
            .map(|a| format!("{}.attributes -> 'startDate'", a)),
        "campaign_endDate" => alias_map
            .get("campaigns_latest")
            .map(|a| format!("{}.attributes -> 'endDate'", a)),
        "offer_id" => alias_map.get("offers_latest").map(|a| format!("{}.id", a)),
        "offer_name" => alias_map
            .get("offers_latest")
            .map(|a| format!("{}.name", a)),
        "workflow_status" => alias_map
            .get("offers_latest")
            .map(|a| format!("{}.status", a)),
        "product_id" => alias_map
            .get("offer_products")
            .map(|a| format!("{}.product_id", a)),
        "product_name" => alias_map
            .get("products_latest")
            .map(|a| format!("{}.name", a)),
        "price_id" => alias_map
            .get("offer_products")
            .map(|a| format!("{}.price_id", a)),
        "price_amount" | "amount" => alias_map.get("prices").map(|a| format!("{}.amount", a)),
        "currency" => alias_map.get("prices").map(|a| format!("{}.currency", a)),
        "promo_type" => Some("promo_type".to_string()), // Special: resolved in filter translation
        _ => None,
    };
    if semantic_result.is_some() {
        return semantic_result;
    }

    // Priority 2: Check json_paths in schema (source of truth for JSONB attributes)
    // Try both exact match and camelCase conversion (e.g., package_id → packageId)
    // Array fields: use -> operator, caller will add ->> 0 or use array operators
    // Scalar fields: use ->> operator directly for text extraction
    let field_camel = to_camel_case(field);
    for entity in &cards.entities {
        if let Some(alias) = alias_map.get(&entity.name) {
            for json_path in &entity.json_paths {
                // Extract field name from $.fieldName
                let path_field = json_path.path.trim_start_matches("$.");
                if path_field == field || path_field == field_camel {
                    // Use -> for arrays (caller will add ->> 0 or use array operators)
                    // Use ->> for scalars (direct text extraction)
                    let operator = if json_path.data_type.to_lowercase() == "array" {
                        "->"
                    } else {
                        "->>"
                    };
                    return Some(format!(
                        "{}.{} {} '{}'",
                        alias, json_path.column, operator, path_field
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
                if let Some(col) = entity.columns.iter().find(|c| c.name == field) {
                    // JSONB columns must be cast to text for serialization
                    // When user asks for "legacy" or "attributes", return the whole JSONB as text
                    if col.data_type.to_lowercase() == "jsonb" {
                        return Some(format!("{}.{}::text", alias, field));
                    } else {
                        return Some(format!("{}.{}", alias, field));
                    }
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

/// Helper to check if a field is used with overlaps operator (multi-value array)
fn is_multivalue_array_field(field: &str, spec: &ReportSpec) -> bool {
    spec.filters
        .iter()
        .any(|f| f.field == field && matches!(f.op, FilterOp::Overlaps))
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
    spec: &ReportSpec,
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

            // For JSONB single-value array fields, add ->> 0 to extract first element
            // Multi-value arrays (used with overlaps) keep the full array
            let expr = if is_jsonb_array_field(&item.field, cards)
                && !is_multivalue_array_field(&item.field, spec)
                && expr.contains("->")
                && !expr.contains("->>")
            {
                format!("{} ->> 0", expr)
            } else {
                expr
            };

            // Auto-generate meaningful aliases for better column names
            let alias = if item.alias.is_none() {
                // Generate human-readable aliases based on field names (using snake_case)
                match item.field.as_str() {
                    // Explicit semantic field names - keep as-is (already disambiguated)
                    "offer_id" | "offer_name" | "product_id" | "product_name" | "price_id"
                    | "price_amount" | "campaign_id" | "campaign_name" => Some(item.field.clone()),
                    // Field aliases that need explicit naming
                    "amount" => Some("price_amount".to_string()),
                    "currency" => Some("currency".to_string()),
                    // Campaign fields already have campaign_ prefix, keep as-is
                    "campaign_startDate" | "campaign_endDate" => {
                        // Convert to snake_case: campaign_startDate -> campaign_start_date
                        let snake = item
                            .field
                            .replace("startDate", "start_date")
                            .replace("endDate", "end_date");
                        Some(snake)
                    }
                    // Offer fields - add "offer_" prefix and use snake_case
                    "id" => Some("offer_id".to_string()),
                    "name" => Some("offer_name".to_string()),
                    "startDate" => Some("offer_start_date".to_string()),
                    "endDate" => Some("offer_end_date".to_string()),
                    // For other fields with JSON extractions, convert camelCase to snake_case
                    _ if expr.contains("->") || expr.contains("->>") => {
                        // Convert camelCase to snake_case
                        let snake = to_snake_case(&item.field);
                        Some(snake)
                    }
                    // No alias needed for direct column access
                    _ => None,
                }
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

/// Helper to check if a field is a JSONB array field
/// Returns true only if the field is defined in json_paths with data_type = "array"
fn is_jsonb_array_field(field: &str, cards: &SchemaCards) -> bool {
    let field_camel = to_camel_case(field);

    // Handle campaign_ prefix by also checking without it
    let field_without_prefix = field.strip_prefix("campaign_").unwrap_or(field);
    let field_without_prefix_camel = to_camel_case(field_without_prefix);

    for entity in &cards.entities {
        for json_path in &entity.json_paths {
            let path_field = json_path.path.trim_start_matches("$.");
            if path_field == field
                || path_field == field_camel
                || path_field == field_without_prefix
                || path_field == field_without_prefix_camel
            {
                // Check if this specific field is defined as an array type
                // Legacy fields (e.g., hulu_bundle_id) are scalars, not arrays
                // Attributes fields may be arrays or scalars depending on data_type
                return json_path.data_type.to_lowercase() == "array";
            }
        }
    }
    false
}

/// Get the JSONB base path for a field (e.g., "o.attributes -> 'brand'" or "oph.legacy ->> 'hulu_bundle_id'")
fn get_jsonb_base_path(
    field: &str,
    alias_map: &HashMap<String, String>,
    cards: &SchemaCards,
) -> Option<String> {
    let field_camel = to_camel_case(field);
    for entity in &cards.entities {
        if let Some(alias) = alias_map.get(&entity.name) {
            for json_path in &entity.json_paths {
                let path_field = json_path.path.trim_start_matches("$.");
                if path_field == field || path_field == field_camel {
                    // Use -> for arrays (will be used with array operators like @>, ?|)
                    // Use ->> for scalars (will be used with standard comparison operators)
                    let operator = if json_path.data_type.to_lowercase() == "array" {
                        "->"
                    } else {
                        "->>"
                    };
                    return Some(format!(
                        "{}.{} {} '{}'",
                        alias, json_path.column, operator, path_field
                    ));
                }
            }
        }
    }
    None
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

    // Check if this is a JSONB array field
    let is_array = is_jsonb_array_field(&filter.field, cards);

    match filter.op {
        FilterOp::Eq => {
            // Handle null checks with IS NULL
            if filter.value.is_null() {
                return Ok(format!("{} IS NULL", column_sql));
            }

            // For JSONB array fields, use @> containment operator
            if is_array {
                let jsonb_base =
                    get_jsonb_base_path(&filter.field, alias_map, cards).ok_or_else(|| {
                        CompilerError::InvalidFilter {
                            field: filter.field.clone(),
                        }
                    })?;

                let rhs = match &filter.value {
                    Value::String(s) => {
                        // For brand field, use ILIKE for case-insensitive comparison
                        if filter.field == "brand" {
                            return Ok(format!(
                                "{} ->> 0 ILIKE '{}'",
                                jsonb_base,
                                s.replace('\'', "''")
                            ));
                        }
                        format!("'[\"{}\"]'", s.replace('\'', "''").replace('"', "\\\""))
                    }
                    _ => {
                        return Err(CompilerError::InvalidFilter {
                            field: filter.field.clone(),
                        })
                    }
                };
                return Ok(format!("{} @> {}", jsonb_base, rhs));
            }

            // For non-array fields, use standard equality
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

            // For JSONB array fields, use ?| operator (contains any of)
            if is_array {
                let jsonb_base =
                    get_jsonb_base_path(&filter.field, alias_map, cards).ok_or_else(|| {
                        CompilerError::InvalidFilter {
                            field: filter.field.clone(),
                        }
                    })?;

                // For brand field, use LOWER for case-insensitive IN comparison
                if filter.field == "brand" {
                    let lowered_values = arr
                        .iter()
                        .filter_map(|v| match v {
                            Value::String(s) => {
                                Some(format!("'{}'", s.to_lowercase().replace('\'', "''")))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Ok(format!(
                        "LOWER({} ->> 0) IN ({})",
                        jsonb_base, lowered_values
                    ));
                }

                let elements_sql = arr
                    .iter()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                return Ok(format!("{} ?| ARRAY[{}]", jsonb_base, elements_sql));
            }

            // For non-array fields, use standard IN
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
            // For brand field with overlaps, use case-insensitive comparison
            if filter.field == "brand" {
                let lowered_values = arr
                    .iter()
                    .filter_map(|v| match v {
                        Value::String(s) => {
                            Some(format!("'{}'", s.to_lowercase().replace('\'', "''")))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Ok(format!(
                    "LOWER({} ->> 0) IN ({})",
                    column_sql, lowered_values
                ));
            }

            let elements_sql = arr
                .iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", ");

            // Check if this is a JSONB field (indicated by -> operator in column_sql)
            if column_sql.contains("->") {
                // JSONB field: use ?| operator (JSONB contains any of)
                // Wrap in parentheses to avoid operator precedence issues
                Ok(format!("({} ?| ARRAY[{}])", column_sql, elements_sql))
            } else {
                // Direct array column: use && operator
                Ok(format!("{} && ARRAY[{}]", column_sql, elements_sql))
            }
        }
        FilterOp::Gt | FilterOp::Gte | FilterOp::Lt | FilterOp::Lte => {
            // Comparison operators (dates or numbers)
            let op_str = match filter.op {
                FilterOp::Gt => ">",
                FilterOp::Gte => ">=",
                FilterOp::Lt => "<",
                FilterOp::Lte => "<=",
                _ => unreachable!(),
            };

            // For JSONB array fields, add ->> 0 to extract first element for comparison
            let lhs = if is_array && column_sql.contains("->") && !column_sql.contains("->>") {
                format!("{} ->> 0", column_sql)
            } else {
                column_sql
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

            // Special handling for endDate fields with >= or > operators
            // NULL endDate means unlimited/ongoing, so should be included
            let is_end_date = filter.field == "endDate" || filter.field == "campaign_endDate";
            if is_end_date && matches!(filter.op, FilterOp::Gte | FilterOp::Gt) {
                Ok(format!("({} {} {} OR {} IS NULL)", lhs, op_str, rhs, lhs))
            } else {
                Ok(format!("{} {} {}", lhs, op_str, rhs))
            }
        }
        FilterOp::IsNull => {
            // Check if value is NULL
            Ok(format!("{} IS NULL", column_sql))
        }
        FilterOp::IsNotNull => {
            // Check if value is NOT NULL
            Ok(format!("{} IS NOT NULL", column_sql))
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

fn resolve_entity<'a>(
    field: &str,
    cards: &'a SchemaCards,
    primary_entity: Option<&str>,
) -> Option<&'a str> {
    // 0. If primary_entity is specified and the field exists in that entity, prefer it
    // This solves ambiguity when the same field exists in multiple tables
    if let Some(primary) = primary_entity {
        // Check if field exists in primary entity's columns
        if let Some(entity) = cards.entities.iter().find(|e| e.name == primary) {
            if entity.columns.iter().any(|col| col.name == field) {
                return Some(entity.name.as_str());
            }
            // Check json_paths in primary entity
            let field_camel = to_camel_case(field);
            for json_path in &entity.json_paths {
                let path_field = json_path.path.trim_start_matches("$.");
                if path_field == field || path_field == field_camel {
                    return Some(entity.name.as_str());
                }
            }
        }
    }

    // 1. Hard-coded mapping for the campaigns_offers workspace
    match field {
        // partner-level field
        "partnership_id" => return Some("partners"),
        // campaign-level fields (brand requires campaigns_latest)
        "campaign_id" | "campaign_name" | "brand" => return Some("campaigns_latest"),
        // offer-level fields (direct columns)
        "offer_id" | "offer_name" | "workflow_status" | "countries" | "package_id" => {
            return Some("offers_latest")
        }
        // product-level fields (product_id exists in offer_products bridge table)
        "product_id" => return Some("offer_products"),
        "product_name" => return Some("products_latest"),
        // price-level fields
        "price_id" => return Some("offer_products"), // FK in offer_products, not prices.id
        "price_amount" | "amount" | "currency" => return Some("prices"),
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

    // Extract primary_entity from spec (if specified)
    let primary_entity = spec.primary_entity.as_deref();

    let select_entities = spec
        .select
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards, primary_entity))
        .collect::<Vec<_>>();
    let filter_entities = spec
        .filters
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards, primary_entity))
        .collect::<Vec<_>>();
    let order_by_entities = spec
        .order_by
        .iter()
        .map(|s| resolve_entity(&s.field, schema_cards, primary_entity))
        .collect::<Vec<_>>();

    let mut required_entities: Vec<_> = select_entities
        .iter()
        .chain(filter_entities.iter())
        .chain(order_by_entities.iter())
        .collect();

    // Add bridge tables when needed for joins
    // If we have both offers_latest and campaigns_latest, add campaign_offers
    let has_offers = required_entities
        .iter()
        .any(|e| e.as_ref() == Some(&"offers_latest"));
    let has_campaigns = required_entities
        .iter()
        .any(|e| e.as_ref() == Some(&"campaigns_latest"));

    if has_offers && has_campaigns {
        required_entities.push(&Some("campaign_offers"));
    }

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
                    "products_latest" => "pl",
                    "prices" => "pr",
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
    let projections = translate_projections(&spec.select, &alias_map, &reg.cards, spec)?;
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
