use super::llm::{LlmClient, LlmRequest, LlmMessage, LlmRole, LlmOutput, LlmError};
use super::planner::{Planner, PlannerContext, PlannerResult, PlannerError, ReportSpecDraft};
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
        if trimmed.starts_with('{') {
            return Ok(trimmed.to_string());
        }
        
        // Try to find JSON block in response
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                if end > start {
                    return Ok(trimmed[start..=end].to_string());
                }
            }
        }
        
        Err(LlmError::InvalidFormat("No valid JSON found in response".to_string()))
    }
    
    /// Validate that all required fields are present
    fn validate_required_fields(&self, parsed: &serde_json::Value) -> Result<(), LlmError> {
        let required_fields = ["report_spec", "assumptions", "open_questions"];
        
        for field in &required_fields {
            if !parsed.get(field).is_some() {
                return Err(LlmError::MissingField(field.to_string()));
            }
        }
        
        // Validate report_spec has required structure
        let report_spec = parsed.get("report_spec").unwrap();
        let spec_required = ["version", "workspace", "select", "filters", "order_by", "mode"];
        
        for field in &spec_required {
            if !report_spec.get(field).is_some() {
                return Err(LlmError::MissingField(format!("report_spec.{}", field)));
            }
        }
        
        Ok(())
    }
    
    /// Validate ReportSpec structure and constraints
    fn validate_report_spec(&self, spec: &crate::dsl::report_spec::ReportSpec) -> Result<(), LlmError> {
        // Basic validation
        if spec.select.is_empty() {
            return Err(LlmError::InvalidFormat("ReportSpec must have at least one select field".to_string()));
        }
        
        // Version validation
        if spec.version != 1 {
            return Err(LlmError::InvalidFormat(format!("Unsupported ReportSpec version: {}", spec.version)));
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
    fn make_llm_request(&self, system_prompt: String, user_prompt: String) -> Result<LlmOutput, PlannerError> {
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
            max_tokens: Some(2048),
        };

        let response = self.client.complete(request)
            .map_err(|e| PlannerError::InternalError(format!("LLM client error: {}", e)))?;

        self.parse_llm_output(&response.content)
            .map_err(|e| PlannerError::InternalError(format!("Failed to parse LLM output: {}", e)))
    }
}

impl Planner for LlmPlanner {
    fn suggest_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft> {
        let system_prompt = self.build_system_prompt(&ctx);
        let llm_output = self.make_llm_request(system_prompt, prompt.to_string())?;

        Ok(ReportSpecDraft {
            spec: llm_output.report_spec,
            rationale: llm_output.notes,
            assumptions: llm_output.assumptions,
        })
    }

    fn revise_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
        diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft> {
        // For revision, we need the previous spec - this is a limitation of current interface
        // For now, we'll treat this as a fresh attempt with diagnostic context
        let system_prompt = format!(
            r#"You are a ReportSpec generator. The previous attempt failed compilation.

COMPILER ERRORS: {:?}

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- Fix the errors mentioned above
- No SQL generation

WORKSPACE: {}
AVAILABLE_TABLES: {:?}
AVAILABLE_FIELDS: {:?}

OUTPUT FORMAT:
{{
  "report_spec": {{ ... }},
  "assumptions": ["..."],
  "open_questions": ["..."],
  "notes": "..."
}}"#,
            diagnostics, ctx.workspace, ctx.available_tables, ctx.available_fields
        );

        let llm_output = self.make_llm_request(system_prompt, prompt.to_string())?;

        Ok(ReportSpecDraft {
            spec: llm_output.report_spec,
            rationale: llm_output.notes,
            assumptions: llm_output.assumptions,
        })
    }
}