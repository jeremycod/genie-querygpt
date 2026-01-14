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
        let mut tables: Vec<TableSummary> = registry
            .cards
            .entities
            .iter()
            .map(|entity| {
                // Generate table alias (first letter of each word)
                let alias = Self::generate_alias(&entity.name);

                // Convert columns to field summaries
                // Skip promoted columns that have JSONB equivalents (we'll expose JSONB version instead)
                let skip_promoted_columns = ["start_date", "end_date", "status", "countries"];

                // Get list of JSONB columns that have json_paths defined
                // Add warning to discourage direct selection
                let jsonb_columns_with_paths: std::collections::HashSet<&str> = entity
                    .json_paths
                    .iter()
                    .map(|jp| jp.column.as_str())
                    .collect();

                let mut fields: Vec<FieldSummary> = entity
                    .columns
                    .iter()
                    .filter(|col| !skip_promoted_columns.contains(&col.name.as_str()))
                    .map(|col| {
                        let mut description = col.description.clone();

                        // Add warning for JSONB columns with json_paths
                        if jsonb_columns_with_paths.contains(col.name.as_str()) {
                            let extracted_fields: Vec<String> = entity
                                .json_paths
                                .iter()
                                .filter(|jp| jp.column == col.name)
                                .map(|jp| jp.path.trim_start_matches("$.").to_string())
                                .take(8) // Show first 8 fields
                                .collect();

                            if !extracted_fields.is_empty() {
                                description = format!(
                                    "⚠️ DO NOT USE FOR FILTERING. Use extracted fields instead: {}. This raw JSONB column should only be selected if user explicitly asks for the complete raw JSON object.",
                                    extracted_fields.join(", ")
                                );
                            }
                        }

                        FieldSummary {
                            name: col.name.clone(),
                            field_type: col.data_type.clone(),
                            nullable: col.nullable,
                            description: Some(description),
                            enum_values: None,
                        }
                    })
                    .collect();

                // Add JSONB paths as queryable fields (use camelCase field names)
                for json_path in &entity.json_paths {
                    // Extract field name from $.fieldName
                    let field_name = json_path.path.trim_start_matches("$.");

                    // Add note about source column for context
                    let enhanced_description = if json_path.column == "legacy" || json_path.column == "attributes" {
                        format!(
                            "{} (extracted from {} JSONB column)",
                            json_path.description,
                            json_path.column
                        )
                    } else {
                        json_path.description.clone()
                    };

                    fields.push(FieldSummary {
                        name: field_name.to_string(),
                        field_type: json_path.data_type.clone(),
                        nullable: true, // JSONB fields are always nullable
                        description: Some(enhanced_description),
                        enum_values: None,
                    });
                }

                // RESILIENCE: Detect entities with JSONB columns but no json_paths defined
                // This indicates incomplete schema cards that need to be updated
                let has_jsonb_columns = entity
                    .columns
                    .iter()
                    .any(|col| col.data_type.to_lowercase() == "jsonb");

                if has_jsonb_columns && entity.json_paths.is_empty() {
                    // Log warning for developers
                    tracing::warn!(
                        entity = %entity.name,
                        "Entity has JSONB columns but no json_paths defined. \
                        JSONB fields will not be queryable. \
                        Update schema cards to add json_paths for attributes."
                    );

                    // Add a synthetic field to inform the LLM about this limitation
                    fields.push(FieldSummary {
                        name: "_schema_note".to_string(),
                        field_type: "note".to_string(),
                        nullable: true,
                        description: Some(
                            "⚠️ This table has JSONB columns (attributes) but no queryable fields defined. \
                            If you need to filter by attributes, add the requirement to 'open_questions'."
                                .to_string(),
                        ),
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

        // Add derived fields as queryable fields (they appear as regular fields to the LLM)
        // Derived fields are cross-table computed fields that can be used in queries
        for derived_field in &registry.cards.derived_fields {
            // Extract the table name from the first depends_on field
            // E.g., "products_latest.name" -> "products_latest"
            if let Some(first_dep) = derived_field.depends_on.first() {
                if let Some(table_name) = first_dep.split('.').next() {
                    // Find the table and add this derived field to it
                    if let Some(table) = tables.iter_mut().find(|t| t.name == table_name) {
                        table.fields.push(FieldSummary {
                            name: derived_field.name.clone(),
                            field_type: "derived".to_string(),
                            nullable: true, // Derived fields are typically nullable
                            description: Some(format!(
                                "{} (derived field)",
                                derived_field.description
                            )),
                            enum_values: None,
                        });
                    }
                }
            }
        }

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
