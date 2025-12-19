use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaCards {
    pub version: String,
    pub database: String,
    pub workspace: String,
    pub entities: Vec<EntityCard>,
    pub join_graph: JoinGraph,
    pub derived_fields: Vec<DerivedField>,
    pub conventions: Conventions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conventions {
    pub profile_column: String,
    pub version_column: String,
    pub deleted_column: String,
    pub latest_views: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCard {
    pub name: String,
    pub kind: EntityKind,
    pub description: String,
    pub primary_key: Vec<String>,
    pub columns: Vec<ColumnCard>,
    pub json_paths: Vec<JsonPathCard>,
    pub common_filters: Vec<FilterHint>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Table,
    MaterializedView,
    View,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnCard {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub description: String,
    pub pii: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPathCard {
    pub column: String,
    pub path: String,
    pub data_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterHint {
    pub name: String,
    pub sql: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<JoinEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinEdge {
    pub from: String,
    pub to: String,
    pub join_type: String, // "inner" | "left"
    pub on: Vec<String>,   // list of equality predicates as strings
    pub cardinality: String, // "1:1" | "1:n" | "n:1" | "n:n"
    pub safe: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedField {
    pub name: String,
    pub sql: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    pub workspace: String,
    pub description: String,
    pub schema_cards_path: String,
    pub exemplar_sql_dir: String,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
}
