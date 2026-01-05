use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::report_spec::ReportSpec;
use super::planner::PlannerContext;

/// Prompt template builder for LLM interactions
pub struct PromptTemplates;

impl PromptTemplates {
    /// Generate system prompt for initial ReportSpec generation
    pub fn system_prompt(ctx: &PlannerContext) -> String {
        format!(
            r#"You are a ReportSpec generator. Generate valid JSON only.

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- No SQL generation
- If unsure, add to open_questions

WORKSPACE: {}
AVAILABLE_TABLES: {}
AVAILABLE_FIELDS: {}

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [{{"field": "field_name", "alias": null}}],
    "filters": [],
    "order_by": [],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "optional explanation"
}}

IMPORTANT: Only output valid JSON. No explanations outside the JSON structure."#,
            ctx.workspace,
            ctx.available_tables.join(", "),
            ctx.available_fields.join(", "),
            ctx.workspace
        )
    }

    /// Generate revision prompt for fixing compilation errors
    pub fn revision_prompt(
        original_prompt: &str,
        previous_spec: &ReportSpec,
        diagnostics: &CompilerDiagnostics,
        ctx: &PlannerContext,
    ) -> String {
        format!(
            r#"Previous attempt failed compilation. Fix the ReportSpec.

ORIGINAL PROMPT: {}
PREVIOUS SPEC: {}
COMPILER ERRORS: {}

WORKSPACE: {}
AVAILABLE_TABLES: {}
AVAILABLE_FIELDS: {}

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [{{"field": "field_name", "alias": null}}],
    "filters": [],
    "order_by": [],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "explanation of fixes made"
}}

IMPORTANT: Fix the errors and output only valid JSON. No explanations outside the JSON structure."#,
            original_prompt,
            serde_json::to_string_pretty(previous_spec).unwrap_or_default(),
            format!("{:?}", diagnostics),
            ctx.workspace,
            ctx.available_tables.join(", "),
            ctx.available_fields.join(", "),
            ctx.workspace
        )
    }

    /// Generate user prompt with natural language request
    pub fn user_prompt(natural_language: &str) -> String {
        format!("Generate a ReportSpec for this request: {}", natural_language)
    }
}