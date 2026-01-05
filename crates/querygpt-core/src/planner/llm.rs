use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Core LLM client abstraction for generating ReportSpecs
pub trait LlmClient: Send + Sync {
    fn complete(&self, req: LlmRequest) -> anyhow::Result<LlmResponse>;
}

/// Request structure for LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

/// Individual message in LLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

/// Message roles for LLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

/// Response from LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: Option<LlmUsage>,
}

/// Token usage information from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Parsed output from LLM following strict JSON contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOutput {
    pub report_spec: crate::dsl::report_spec::ReportSpec,
    pub assumptions: Vec<String>,
    pub open_questions: Vec<String>,
    pub notes: Option<String>,
}

/// Tracing information for planner operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerTrace {
    pub model: String,
    pub attempts: usize,
    pub revisions_occurred: bool,
    pub final_status: CompilationStatus,
    pub timestamp: SystemTime,
}

/// Compilation status for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilationStatus {
    Success,
    Failed,
    MaxAttemptsReached,
}

/// Error types for LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("failed to parse LLM output as JSON: {0}")]
    JsonParseError(String),
    
    #[error("LLM output missing required field: {0}")]
    MissingField(String),
    
    #[error("LLM client error: {0}")]
    ClientError(String),
    
    #[error("invalid LLM response format: {0}")]
    InvalidFormat(String),
}