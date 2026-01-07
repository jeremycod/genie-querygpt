use super::planner::PlannerContext;
use super::schema_summary::SchemaSummary;
use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::report_spec::ReportSpec;

/// Prompt template builder for LLM interactions
pub struct PromptTemplates;

impl PromptTemplates {
    /// Generate system prompt for initial ReportSpec generation
    pub fn system_prompt(ctx: &PlannerContext) -> String {
        let schema_info = Self::format_schema_summary(&ctx.schema_summary);
        let examples_info = Self::format_examples(&ctx.examples);
        let constraints_info = Self::format_constraints(&ctx.constraints);

        format!(
            r#"You are a ReportSpec generator. Generate valid JSON only.

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- No SQL generation
- If unsure, add to open_questions
{}

WORKSPACE: {}
{}
{}

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [{{"field": "actual_field_from_schema", "alias": null}}],
    "filters": [{{"field": "field_name", "op": "eq", "value": "example_value"}}],
    "order_by": [{{"field": "field_name", "dir": "asc"}}],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "optional explanation"
}}

CRITICAL RULES:
1. "select" array MUST contain at least one field from the SCHEMA SUMMARY above
2. All field names MUST exist in the schema (see SCHEMA SUMMARY section)
3. Use lowercase for "op" values: "eq", "in", "overlaps", "gte", "lte"
4. Use lowercase for "dir" values: "asc", "desc"
5. "filters" and "order_by" can be empty arrays [] if not needed
6. Only output valid JSON - no explanations outside the JSON structure

FILTER OPERATORS (lowercase):
- "eq": Equal to
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields)
- "gte": Greater than or equal
- "lte": Less than or equal"#,
            constraints_info, ctx.workspace, schema_info, examples_info, ctx.workspace
        )
    }

    /// Generate revision prompt for fixing compilation errors
    pub fn revision_prompt(
        original_prompt: &str,
        previous_spec: &ReportSpec,
        diagnostics: &CompilerDiagnostics,
        ctx: &PlannerContext,
    ) -> String {
        let schema_info = Self::format_schema_summary(&ctx.schema_summary);
        let constraints_info = Self::format_constraints(&ctx.constraints);

        format!(
            r#"Previous attempt failed compilation. Fix the ReportSpec.

ORIGINAL PROMPT: {}
PREVIOUS SPEC: {}
COMPILER ERRORS: {}

CONSTRAINTS:
{}

WORKSPACE: {}
{}

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [{{"field": "actual_field_from_schema", "alias": null}}],
    "filters": [{{"field": "field_name", "op": "eq", "value": "example_value"}}],
    "order_by": [{{"field": "field_name", "dir": "asc"}}],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "explanation of fixes made"
}}

CRITICAL RULES:
1. "select" array MUST contain at least one field from the SCHEMA SUMMARY above
2. All field names MUST exist in the schema (see SCHEMA SUMMARY section)
3. Use lowercase for "op" values: "eq", "in", "overlaps", "gte", "lte"
4. Use lowercase for "dir" values: "asc", "desc"
5. "filters" and "order_by" can be empty arrays [] if not needed

FILTER OPERATORS (lowercase):
- "eq": Equal to
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields)
- "gte": Greater than or equal
- "lte": Less than or equal

IMPORTANT: Fix the errors and output only valid JSON. No explanations outside the JSON structure."#,
            original_prompt,
            serde_json::to_string_pretty(previous_spec).unwrap_or_default(),
            format!("{:?}", diagnostics),
            constraints_info,
            ctx.workspace,
            schema_info,
            ctx.workspace
        )
    }

    /// Generate user prompt with natural language request
    pub fn user_prompt(natural_language: &str) -> String {
        format!(
            "Generate a ReportSpec for this request: {}",
            natural_language
        )
    }

    /// Format schema summary for LLM context
    fn format_schema_summary(schema: &SchemaSummary) -> String {
        let mut result = String::from("SCHEMA SUMMARY:\n");

        // Format tables and fields
        for table in &schema.tables {
            result.push_str(&format!("Table: {} (alias: {})\n", table.name, table.alias));
            if let Some(desc) = &table.description {
                result.push_str(&format!("  Description: {}\n", desc));
            }
            result.push_str("  Fields:\n");
            for field in &table.fields {
                let nullable = if field.nullable {
                    "nullable"
                } else {
                    "required"
                };
                result.push_str(&format!(
                    "    - {} ({}, {})\n",
                    field.name, field.field_type, nullable
                ));
                if let Some(desc) = &field.description {
                    result.push_str(&format!("      Description: {}\n", desc));
                }
                if let Some(enum_vals) = &field.enum_values {
                    result.push_str(&format!("      Values: [{}]\n", enum_vals.join(", ")));
                }
            }
            result.push('\n');
        }

        // Format enums
        if !schema.enums.is_empty() {
            result.push_str("ENUMS:\n");
            for enum_def in &schema.enums {
                result.push_str(&format!(
                    "  {}: [{}]\n",
                    enum_def.name,
                    enum_def.values.join(", ")
                ));
                if let Some(desc) = &enum_def.description {
                    result.push_str(&format!("    Description: {}\n", desc));
                }
            }
        }

        result
    }

    /// Format examples for LLM context
    fn format_examples(examples: &[super::schema_summary::ExamplePair]) -> String {
        if examples.is_empty() {
            return String::new();
        }

        let mut result = String::from("EXAMPLES:\n");
        for (i, example) in examples.iter().enumerate() {
            result.push_str(&format!("Example {}:\n", i + 1));
            result.push_str(&format!("  Prompt: {}\n", example.prompt));
            result.push_str(&format!("  Description: {}\n", example.description));
            if let Ok(spec_json) = serde_json::to_string_pretty(&example.spec) {
                result.push_str(&format!("  ReportSpec: {}\n", spec_json));
            }
            result.push('\n');
        }

        result
    }

    /// Format constraints for LLM context
    fn format_constraints(constraints: &super::schema_summary::PlannerConstraints) -> String {
        let mut result = String::new();

        if constraints.max_select_fields > 0 {
            result.push_str(&format!(
                "- Maximum {} select fields\n",
                constraints.max_select_fields
            ));
        }
        if constraints.max_filters > 0 {
            result.push_str(&format!("- Maximum {} filters\n", constraints.max_filters));
        }
        if !constraints.forbidden_patterns.is_empty() {
            result.push_str(&format!(
                "- Forbidden patterns: {}\n",
                constraints.forbidden_patterns.join(", ")
            ));
        }

        result
    }
}
