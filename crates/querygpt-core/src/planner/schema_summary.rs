use crate::dsl::report_spec::ReportSpec;
use crate::schema::registry::SchemaRegistry;
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
                fields: vec![FieldSummary {
                    name: "id".to_string(),
                    field_type: "uuid".to_string(),
                    nullable: false,
                    description: Some("Primary key".to_string()),
                    enum_values: None,
                }],
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

    /// Build a SchemaSummary from a SchemaRegistry
    pub fn from_registry(registry: &SchemaRegistry) -> Self {
        // Convert entities to table summaries
        let tables = registry
            .cards
            .entities
            .iter()
            .map(|entity| {
                // Generate table alias (first letter of each word)
                let alias = Self::generate_alias(&entity.name);

                // Convert columns to field summaries
                // Skip promoted columns that have JSONB equivalents (we'll expose JSONB version instead)
                let skip_promoted_columns = ["start_date", "end_date", "status", "countries"];
                let mut fields: Vec<FieldSummary> = entity
                    .columns
                    .iter()
                    .filter(|col| !skip_promoted_columns.contains(&col.name.as_str()))
                    .map(|col| FieldSummary {
                        name: col.name.clone(),
                        field_type: col.data_type.clone(),
                        nullable: col.nullable,
                        description: Some(col.description.clone()),
                        enum_values: None, // Could be enhanced later
                    })
                    .collect();

                // Add JSONB paths as queryable fields (use camelCase field names)
                for json_path in &entity.json_paths {
                    // Extract field name from $.fieldName
                    let field_name = json_path.path.trim_start_matches("$.");
                    fields.push(FieldSummary {
                        name: field_name.to_string(),
                        field_type: json_path.data_type.clone(),
                        nullable: true, // JSONB fields are always nullable
                        description: Some(json_path.description.clone()),
                        enum_values: None,
                    });
                }

                TableSummary {
                    name: entity.name.clone(),
                    alias,
                    fields,
                    description: Some(entity.description.clone()),
                }
            })
            .collect();

        // Convert join graph to relationships (simplified for now)
        let relationships = registry
            .cards
            .join_graph
            .edges
            .iter()
            .map(|edge| {
                // Parse the first ON condition to extract field names
                // Format is like "table1.field1 = table2.field2"
                let (from_field, to_field) = edge
                    .on
                    .first()
                    .and_then(|on_clause| {
                        let parts: Vec<&str> = on_clause.split('=').collect();
                        if parts.len() == 2 {
                            let from = parts[0].trim().split('.').nth(1)?.trim();
                            let to = parts[1].trim().split('.').nth(1)?.trim();
                            Some((from.to_string(), to.to_string()))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| ("id".to_string(), "id".to_string()));

                let relationship_type = match edge.cardinality.as_str() {
                    "1:1" => RelationshipType::OneToOne,
                    "1:n" => RelationshipType::OneToMany,
                    "n:1" => RelationshipType::ManyToOne,
                    "n:n" => RelationshipType::ManyToMany,
                    _ => RelationshipType::OneToMany,
                };

                JoinRelationship {
                    from_table: edge.from.clone(),
                    from_field,
                    to_table: edge.to.clone(),
                    to_field,
                    relationship_type,
                }
            })
            .collect();

        Self {
            tables,
            relationships,
            enums: vec![], // Could be extracted from schema metadata later
        }
    }

    /// Generate a short alias for a table name
    /// Examples: "offers_latest" -> "o", "campaigns_latest" -> "c"
    fn generate_alias(table_name: &str) -> String {
        // Take first letter of first significant word (skip "latest", "active", etc.)
        let parts: Vec<&str> = table_name.split('_').collect();
        let significant_part = parts
            .iter()
            .find(|&part| *part != "latest" && *part != "active")
            .unwrap_or(&parts[0]);

        significant_part.chars().next().unwrap_or('t').to_string()
    }
}
