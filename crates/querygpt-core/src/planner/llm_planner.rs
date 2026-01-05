use super::llm::{LlmClient, LlmRequest, LlmMessage, LlmRole, LlmOutput, LlmError};
use super::planner::{Planner, PlannerContext, PlannerResult, PlannerError, ReportSpecDraft};
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
    fn parse_llm_output(&self, raw: &str) -> Result<LlmOutput, LlmError> {
        // Gate A: JSON Parse + Strict Deserialization
        let parsed: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| LlmError::JsonParseError(e.to_string()))?;

        // Validate required fields exist
        if !parsed.get("report_spec").is_some() {
            return Err(LlmError::MissingField("report_spec".to_string()));
        }

        // Deserialize to structured output
        let output: LlmOutput = serde_json::from_value(parsed)
            .map_err(|e| LlmError::InvalidFormat(e.to_string()))?;

        Ok(output)
    }

    /// Generate system prompt for initial spec generation
    fn build_system_prompt(&self, ctx: &PlannerContext) -> String {
        format!(
            r#"You are a ReportSpec generator. Generate valid JSON only.

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- No SQL generation
- If unsure, add to open_questions

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
            ctx.workspace, ctx.available_tables, ctx.available_fields
        )
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
        format!(
            r#"Previous attempt failed compilation. Fix the ReportSpec.

ORIGINAL PROMPT: {}
PREVIOUS SPEC: {}
COMPILER ERRORS: {:?}

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
            original_prompt,
            serde_json::to_string_pretty(previous_spec).unwrap_or_default(),
            diagnostics,
            ctx.workspace,
            ctx.available_tables,
            ctx.available_fields
        )
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
                    content: user_prompt,
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