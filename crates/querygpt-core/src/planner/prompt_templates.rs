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
    "select": [
      {{"field": "id", "alias": null}},
      {{"field": "name", "alias": null}},
      {{"field": "brand", "alias": null}}
    ],
    "filters": [
      {{"field": "brand", "op": "eq", "value": "ESPN"}},
      {{"field": "start_date", "op": "gte", "value": "2025-01-01"}}
    ],
    "order_by": [{{"field": "id", "dir": "asc"}}],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "optional explanation"
}}

FIELD NAME EXAMPLES (use these patterns):
✅ CORRECT: "id", "name", "brand", "start_date", "end_date", "status", "countries"
❌ WRONG: "o.id", "c.name", "campaigns_latest.brand", "offers.start_date"

CRITICAL RULES (YOU MUST FOLLOW THESE):
⚠️  SCHEMA CORRECTNESS:
1. "select" array MUST NOT be empty - include at least one field
2. ALL field names MUST exist in the SCHEMA SUMMARY above (verify each field!)
3. NEVER use field names not listed in the schema
4. If a field is used in "order_by", it MUST also appear in "select"
5. CRITICAL: Use ONLY the logical field names from the schema (e.g., "brand", "id", "name")
6. NEVER use SQL-qualified names (❌ WRONG: "o.id", "c.brand" ✅ CORRECT: "id", "brand")

⚠️  FORMAT CORRECTNESS:
5. Use lowercase for "op" values: "eq", "in", "overlaps", "gt", "gte", "lt", "lte"
6. Use lowercase for "dir" values: "asc", "desc"
7. "filters" and "order_by" can be empty arrays [] if not needed
8. Only output valid JSON - no explanations outside the JSON structure
9. CRITICAL: Close all arrays with ] before closing objects with }}
10. For long country arrays, maintain valid JSON structure

⚠️  COMMON MISTAKES TO AVOID:
- ❌ Empty "select" array
- ❌ Using fields not in schema
- ❌ ORDER BY fields missing from SELECT
- ❌ Uppercase operator names (use "eq" not "Eq")

FILTER OPERATORS (lowercase only):
- "eq": Equal to (for NULL checks use "eq" with value: null, generates IS NULL)
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields, value must be array)
- "gt": Greater than (for dates, numbers)
- "gte": Greater than or equal (for dates, numbers)
- "lt": Less than (for dates, numbers)
- "lte": Less than or equal (for dates, numbers)

NULL CHECKS:
- To check if field IS NULL: {{"field": "fieldName", "op": "eq", "value": null}}
- NEVER use "isnull" or "is_null" as operators - use "eq" with null value

REGION TO COUNTRY MAPPING:
When users mention regions, expand them to ISO 3166-1 alpha-2 country codes:
- APAC (Asia-Pacific): ["AF","AU","BD","BT","BN","KH","CN","HK","IN","ID","JP","KI","KP","KR","LA","MY","MV","MN","MM","NP","NZ","PK","PG","PH","SG","SB","LK","TW","TH","TL","VU","VN"]
- EMEA (Europe/Middle East/Africa): ["AL","DZ","AD","AO","AM","AT","AZ","BH","BY","BE","BA","BW","BG","BI","CM","CV","CF","TD","KM","CG","HR","CY","CZ","DK","DJ","EG","GQ","ER","EE","ET","FI","FR","GA","GM","GE","DE","GH","GR","GN","GW","HU","IS","IR","IQ","IE","IL","IT","CI","JO","KZ","KE","KW","KG","LV","LB","LS","LR","LY","LI","LT","LU","MK","MG","MW","ML","MT","MR","MU","MD","MC","ME","MA","MZ","NA","NL","NE","NG","NO","OM","PS","PL","PT","QA","RO","RU","RW","ST","SA","SN","RS","SC","SL","SK","SI","SO","ZA","SS","ES","SD","SZ","SE","CH","SY","TJ","TZ","TG","TN","TR","TM","UG","UA","AE","GB","UZ","VA","YE","ZM","ZW"]
- LATAM (Latin America): ["AR","BZ","BO","BR","CL","CO","CR","CU","DO","EC","SV","GT","HT","HN","MX","NI","PA","PY","PE","UY","VE"]
- NA (North America): ["US","CA"]

IMPORTANT: Always use country codes, never use region names as literal values!

BRAND FILTERING (ESPN, DISNEY, STAR, HULU):
When users ask for offers by brand (ESPN, DISNEY, STAR, HULU):
- Use the "brand" field in filters: {{"field": "brand", "op": "eq", "value": "ESPN"}}
- Brand values are uppercase: "ESPN", "DISNEY", "STAR", "HULU"
- The system will automatically handle joins between campaigns and offers
- Example: "ESPN offers" → {{"field": "brand", "op": "eq", "value": "ESPN"}}"#,
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

CRITICAL RULES (YOU MUST FOLLOW THESE):
⚠️  SCHEMA CORRECTNESS:
1. "select" array MUST NOT be empty - include at least one field
2. ALL field names MUST exist in the SCHEMA SUMMARY above (verify each field!)
3. NEVER use field names not listed in the schema
4. If a field is used in "order_by", it MUST also appear in "select"
5. CRITICAL: Use ONLY the logical field names from the schema (e.g., "brand", "id", "name")
6. NEVER use SQL-qualified names (❌ WRONG: "o.id", "c.brand" ✅ CORRECT: "id", "brand")

⚠️  FORMAT CORRECTNESS:
5. Use lowercase for "op" values: "eq", "in", "overlaps", "gt", "gte", "lt", "lte"
6. Use lowercase for "dir" values: "asc", "desc"
7. "filters" and "order_by" can be empty arrays [] if not needed
8. Only output valid JSON - no explanations outside the JSON structure
9. CRITICAL: Close all arrays with ] before closing objects with }}
10. For long country arrays, maintain valid JSON structure

⚠️  COMMON MISTAKES TO AVOID:
- ❌ Empty "select" array
- ❌ Using fields not in schema
- ❌ ORDER BY fields missing from SELECT
- ❌ Uppercase operator names (use "eq" not "Eq")

FILTER OPERATORS (lowercase only):
- "eq": Equal to (for NULL checks use "eq" with value: null, generates IS NULL)
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields, value must be array)
- "gt": Greater than (for dates, numbers)
- "gte": Greater than or equal (for dates, numbers)
- "lt": Less than (for dates, numbers)
- "lte": Less than or equal (for dates, numbers)

NULL CHECKS:
- To check if field IS NULL: {{"field": "fieldName", "op": "eq", "value": null}}
- NEVER use "isnull" or "is_null" as operators - use "eq" with null value

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
