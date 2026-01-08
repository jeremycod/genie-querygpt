use super::llm::{LlmClient, LlmError, LlmMessage, LlmOutput, LlmRequest, LlmRole};
use super::planner::{Planner, PlannerContext, PlannerError, PlannerResult, ReportSpecDraft};
use super::prompt_templates::PromptTemplates;
use crate::compile::diagnostics::CompilerDiagnostics;

/// LLM-powered planner that generates ReportSpecs from natural language
pub struct LlmPlanner {
    client: Box<dyn LlmClient>,
    model: String,
    max_attempts: usize,
}

impl LlmPlanner {
    pub fn new(client: Box<dyn LlmClient>, model: String) -> Self {
        Self {
            client,
            model,
            max_attempts: 3,
        }
    }

    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Parse LLM output following strict JSON contract
    /// Gate A: JSON Parse + Strict Deserialization
    fn parse_llm_output(&self, raw: &str) -> Result<LlmOutput, LlmError> {
        // Step 1: Clean the raw output (remove any non-JSON content)
        let cleaned = self.extract_json_from_response(raw)?;

        // Step 2: Parse as JSON
        let parsed: serde_json::Value = serde_json::from_str(&cleaned)
            .map_err(|e| LlmError::JsonParseError(format!("Invalid JSON: {}", e)))?;

        // Step 3: Validate required fields exist
        self.validate_required_fields(&parsed)?;

        // Step 4: Deserialize to structured output
        let output: LlmOutput = serde_json::from_value(parsed)
            .map_err(|e| LlmError::InvalidFormat(format!("Deserialization failed: {}", e)))?;

        // Step 5: Validate ReportSpec structure
        self.validate_report_spec(&output.report_spec)?;

        Ok(output)
    }

    /// Extract JSON from potentially mixed response content
    fn extract_json_from_response(&self, raw: &str) -> Result<String, LlmError> {
        let trimmed = raw.trim();

        // If it starts with {, assume it's pure JSON
        let json_str = if trimmed.starts_with('{') {
            trimmed.to_string()
        } else {
            // Try to find JSON block in response
            if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.rfind('}') {
                    if end > start {
                        trimmed[start..=end].to_string()
                    } else {
                        return Err(LlmError::InvalidFormat(
                            "No valid JSON found in response".to_string(),
                        ));
                    }
                } else {
                    return Err(LlmError::InvalidFormat(
                        "No valid JSON found in response".to_string(),
                    ));
                }
            } else {
                return Err(LlmError::InvalidFormat(
                    "No valid JSON found in response".to_string(),
                ));
            }
        };

        // Apply JSON repairs for common LLM mistakes
        let repaired = self.repair_json(&json_str);
        if repaired != json_str {
            eprintln!("[DEBUG] Applied JSON repair");
        }
        Ok(repaired)
    }

    /// Repair common JSON formatting mistakes made by LLMs
    fn repair_json(&self, json: &str) -> String {
        // Fix pattern: "value": ["item1","item2",...,"lastitem"}, missing ] before }
        // This happens when LLMs generate long arrays and lose track of brackets

        // Look for the specific pattern where a value array is not closed
        // Pattern: "value": [..."VN"},
        // Should be: "value": [..."VN"]},

        // Strategy: Find all instances of "},\n" after "value": [
        // If we're inside a "value": [ ... context and encounter "},\n      {",
        // that likely means we need to add ] before the }

        let lines: Vec<&str> = json.lines().collect();
        let mut result = String::new();
        let mut in_value_array = false;
        let mut bracket_count = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track if we're inside a "value": [ array
            if trimmed.contains(r#""value":"#) || trimmed.contains(r#""value" :"#) {
                in_value_array = true;
                bracket_count = 0;
            }

            // Count brackets
            bracket_count += line.matches('[').count() as i32;
            bracket_count -= line.matches(']').count() as i32;

            // If we see "}," and we're in a value array with unclosed brackets,
            // and the next line starts a new field, add the missing ]
            if in_value_array && bracket_count > 0 && trimmed.ends_with("},") && i + 1 < lines.len()
            {
                let next_line = lines[i + 1].trim();
                if next_line.starts_with(r#"{"field""#) {
                    // Add the missing ]
                    result.push_str(&line.replace("},", "]},"));
                    result.push('\n');
                    in_value_array = false;
                    bracket_count = 0;
                    eprintln!(
                        "[DEBUG] Repaired missing ] in value array at line {}",
                        i + 1
                    );
                    continue;
                }
            }

            // Reset if we've closed all brackets
            if in_value_array && bracket_count == 0 {
                in_value_array = false;
            }

            result.push_str(line);
            if i < lines.len() - 1 {
                result.push('\n');
            }
        }

        result
    }

    /// Validate that all required fields are present
    fn validate_required_fields(&self, parsed: &serde_json::Value) -> Result<(), LlmError> {
        let required_fields = ["report_spec", "assumptions", "open_questions"];

        for field in &required_fields {
            if parsed.get(field).is_none() {
                return Err(LlmError::MissingField(field.to_string()));
            }
        }

        // Validate report_spec has required structure
        let report_spec = parsed.get("report_spec").unwrap();
        let spec_required = [
            "version",
            "workspace",
            "select",
            "filters",
            "order_by",
            "mode",
        ];

        for field in &spec_required {
            if report_spec.get(field).is_none() {
                return Err(LlmError::MissingField(format!("report_spec.{}", field)));
            }
        }

        Ok(())
    }

    /// Validate ReportSpec structure and constraints
    fn validate_report_spec(
        &self,
        spec: &crate::dsl::report_spec::ReportSpec,
    ) -> Result<(), LlmError> {
        // Basic validation
        if spec.select.is_empty() {
            return Err(LlmError::InvalidFormat(
                "ReportSpec must have at least one select field".to_string(),
            ));
        }

        // Version validation
        if spec.version != 1 {
            return Err(LlmError::InvalidFormat(format!(
                "Unsupported ReportSpec version: {}",
                spec.version
            )));
        }

        Ok(())
    }

    /// Generate system prompt for initial spec generation
    fn build_system_prompt(&self, ctx: &PlannerContext) -> String {
        PromptTemplates::system_prompt(ctx)
    }

    /// Generate revision prompt for fixing compilation errors
    #[allow(dead_code)]
    fn build_revision_prompt(
        &self,
        original_prompt: &str,
        previous_spec: &crate::dsl::report_spec::ReportSpec,
        diagnostics: &CompilerDiagnostics,
        ctx: &PlannerContext,
    ) -> String {
        PromptTemplates::revision_prompt(original_prompt, previous_spec, diagnostics, ctx)
    }

    /// Make LLM request and parse response
    async fn make_llm_request(
        &self,
        system_prompt: String,
        user_prompt: String,
    ) -> Result<LlmOutput, PlannerError> {
        let request = LlmRequest {
            messages: vec![
                LlmMessage {
                    role: LlmRole::System,
                    content: system_prompt,
                },
                LlmMessage {
                    role: LlmRole::User,
                    content: PromptTemplates::user_prompt(&user_prompt),
                },
            ],
            model: self.model.clone(),
            temperature: 0.1, // Low temperature for consistent structured output
            max_tokens: Some(4096), // Increased for long country lists
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(|e| PlannerError::InternalError(format!("LLM client error: {}", e)))?;

        eprintln!("[DEBUG] Raw LLM response:\n{}", response.content);

        self.parse_llm_output(&response.content)
            .map_err(|e| PlannerError::InternalError(format!("Failed to parse LLM output: {}", e)))
    }
}

#[async_trait::async_trait]
impl Planner for LlmPlanner {
    async fn suggest_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        let system_prompt = self.build_system_prompt(&ctx);
        let llm_output = self
            .make_llm_request(system_prompt, prompt.to_string())
            .await?;

        Ok(ReportSpecDraft {
            spec: llm_output.report_spec,
            rationale: llm_output.notes,
            assumptions: llm_output.assumptions,
        })
    }

    async fn revise_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
        diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        // Build revision prompt with diagnostic context
        let system_prompt = format!(
            r#"You are a ReportSpec generator. The previous attempt failed compilation.

COMPILER ERRORS: {}

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- Fix the errors mentioned above
- No SQL generation

🚨 FIELD NAME RULES - PAY ATTENTION 🚨
If error says 'unknown field "o.id"' → Use "id" instead
If error says 'unknown field "c.name"' → Use "campaign_name" instead
If error says 'unknown field "offers_latest.status"' → Use "status" instead
If error says 'unknown field "campaigns_latest.brand"' → Use "brand" instead

NEVER use:
  ❌ "o.id", "o.name", "o.status"
  ❌ "c.id", "c.name", "c.brand"
  ❌ Any field with dots (.) or table prefixes

ALWAYS use:
  ✅ "id", "name", "status", "brand", "startDate", "campaign_id", "campaign_name"

WORKSPACE: {}
AVAILABLE_TABLES: {}
AVAILABLE_FIELDS: {}

REQUIRED OUTPUT FORMAT (COPY THIS STRUCTURE EXACTLY):
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [
      {{"field": "id", "alias": null}},
      {{"field": "name", "alias": null}},
      {{"field": "startDate", "alias": null}}
    ],
    "filters": [
      {{"field": "brand", "op": "eq", "value": "ESPN"}},
      {{"field": "status", "op": "in", "value": ["LIVE"]}},
      {{"field": "startDate", "op": "gte", "value": "2025-01-01"}}
    ],
    "order_by": [],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "explanation of fixes made"
}}

CRITICAL JSON SCHEMA RULES:
- Filter operator key is "op" (not "operator"!)
- Valid "op" values: "eq", "in", "overlaps", "gt", "gte", "lt", "lte" (lowercase!)
- Do NOT add fields like "condition" - they don't exist in the schema
- "filters" is an array of objects with "field", "op", and "value" only

IMPORTANT: Fix the errors and output only valid JSON. No explanations outside the JSON structure."#,
            format!("{:?}", diagnostics),
            ctx.workspace,
            ctx.available_tables.join(", "),
            ctx.available_fields.join(", "),
            ctx.workspace
        );

        let llm_output = self
            .make_llm_request(system_prompt, prompt.to_string())
            .await?;

        Ok(ReportSpecDraft {
            spec: llm_output.report_spec,
            rationale: llm_output.notes,
            assumptions: llm_output.assumptions,
        })
    }
}
