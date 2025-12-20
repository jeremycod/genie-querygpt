use crate::dsl::plan::{IntermediatePlan, PlanTable};
use crate::dsl::report_spec::ReportSpec;
use crate::schema::cards::SchemaCards;
use crate::schema::registry::SchemaRegistry;

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
    let plan_tables = required_entities.iter().filter_map(|e| {
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

    let plan = IntermediatePlan {
        workspace: spec.workspace.clone(),
        tables: plan_tables,
        joins: Vec::new(),
        projections: Vec::new(),
        filters: Vec::new(),
        order_by: Vec::new()
    };
    Ok(plan)
}
