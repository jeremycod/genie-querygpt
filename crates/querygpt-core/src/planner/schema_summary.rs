use crate::dsl::report_spec::ReportSpec;
use serde::{Deserialize, Serialize};

/// Enhanced schema summary for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSummary {
    pub tables: Vec<TableSummary>,
    pub relationships: Vec<JoinRelationship>,
    pub enums: Vec<EnumSummary>,
}

/// Summary of a table with its fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSummary {
    pub name: String,
    pub alias: String,
    pub fields: Vec<FieldSummary>,
    pub description: Option<String>,
}

/// Summary of a field with type and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSummary {
    pub name: String,
    pub field_type: String,
    pub nullable: bool,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

/// Relationship between tables for joins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRelationship {
    pub from_table: String,
    pub from_field: String,
    pub to_table: String,
    pub to_field: String,
    pub relationship_type: RelationshipType,
}

/// Type of relationship between tables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Enum definition with possible values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumSummary {
    pub name: String,
    pub values: Vec<String>,
    pub description: Option<String>,
}

/// Example prompt-to-spec pair for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplePair {
    pub prompt: String,
    pub spec: ReportSpec,
    pub description: String,
}

/// Safety constraints for planner operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConstraints {
    pub max_select_fields: usize,
    pub max_filters: usize,
    pub allowed_workspaces: Vec<String>,
    pub forbidden_patterns: Vec<String>,
}

impl Default for PlannerConstraints {
    fn default() -> Self {
        Self {
            max_select_fields: 20,
            max_filters: 10,
            allowed_workspaces: vec![],
            forbidden_patterns: vec![],
        }
    }
}

impl SchemaSummary {
    /// Create a minimal schema summary for testing
    pub fn minimal(workspace: &str) -> Self {
        Self {
            tables: vec![TableSummary {
                name: format!("{}_latest", workspace),
                alias: workspace.to_string(),
                fields: vec![
                    FieldSummary {
                        name: "id".to_string(),
                        field_type: "uuid".to_string(),
                        nullable: false,
                        description: Some("Primary key".to_string()),
                        enum_values: None,
                    },
                ],
                description: Some(format!("Latest {} data", workspace)),
            }],
            relationships: vec![],
            enums: vec![],
        }
    }

    /// Get all available field names across all tables
    pub fn get_all_fields(&self) -> Vec<String> {
        self.tables
            .iter()
            .flat_map(|table| table.fields.iter().map(|field| field.name.clone()))
            .collect()
    }

    /// Get all available table names
    pub fn get_all_tables(&self) -> Vec<String> {
        self.tables.iter().map(|table| table.name.clone()).collect()
    }
}