use serde::Serialize;

// Each table used in the query, with an alias
#[derive(Debug, Clone, Serialize)]
pub struct PlanTable {
    pub name: String,      // e.g. "offers_latest"
    pub alias: String,     // e.g. "o"
}

// A single join between two tables
#[derive(Debug, Clone, Serialize)]
pub struct PlanJoin {
    pub left_alias: String,    // alias of left table, e.g. "o"
    pub right_alias: String,   // alias of right table, e.g. "c"
    pub join_type: JoinType,   // Inner, Left, etc.
    pub conditions: Vec<JoinCondition>,  // list of equality predicates
}

// Equality predicate for a join
#[derive(Debug, Clone, Serialize)]
pub struct JoinCondition {
    pub left_field: String,    // e.g. "o.id"
    pub right_field: String,   // e.g. "c.campaign_id"
}

// Type of join (inner, left)
#[derive(Debug, Clone, Serialize)]
pub enum JoinType {
    Inner,
    Left,
}

// A projected field in the SELECT clause
#[derive(Debug, Clone, Serialize)]
pub struct PlanProjection {
    pub field: String,        // workspace field name, e.g. "offer_id"
    pub expression: String,   // actual SQL expression, e.g. "o.id"
    pub alias: Option<String>,
}

// A filter predicate in the WHERE clause
#[derive(Debug, Clone, Serialize)]
pub struct PlanFilter {
    pub expression: String,   // actual SQL, e.g. "o.status = 'PUBLISHED'"
}

// A sort directive in the ORDER BY clause
#[derive(Debug, Clone, Serialize)]
pub struct PlanOrder {
    pub expression: String,   // actual SQL, e.g. "p.id"
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

// The overall intermediate plan
#[derive(Debug, Clone, Serialize)]
pub struct IntermediatePlan {
    pub workspace: String,          // e.g. "campaigns_offers"
    pub tables: Vec<PlanTable>,
    pub joins: Vec<PlanJoin>,
    pub projections: Vec<PlanProjection>,
    pub filters: Vec<PlanFilter>,
    pub order_by: Vec<PlanOrder>,

    pub limit: Option<u64>,
    pub offset: Option<u64>,
}
