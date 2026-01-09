use querygpt_core::compile::diagnostics::CompilerDiagnostics;
use querygpt_core::dsl::plan::IntermediatePlan;
use querygpt_core::planner::diff::SpecDiff;
use querygpt_core::planner::planner::{PlannerError, ReportSpecDraft};
use querygpt_core::planner::trace::PlannerTrace;
use serde::{Deserialize, Serialize};

// ============================================================================
// Request Types
// ============================================================================

/// Initial query request from UI client
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// Natural language prompt from user
    pub prompt: String,
    /// Whether to auto-approve changes (skip confirmation)
    #[serde(default)]
    pub auto_approve: bool,
    /// Maximum number of retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: usize,
    /// Optional session ID for continuing a previous session
    pub session_id: Option<String>,
}

fn default_max_attempts() -> usize {
    3
}

/// Confirmation request - user's response to pending changes
#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    /// Session ID for the pending confirmation
    pub session_id: String,
    /// User's decision
    pub action: ConfirmAction,
}

/// User's confirmation action
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConfirmAction {
    Approve,
    Reject,
    Modify { feedback: String },
}

// ============================================================================
// Response Types
// ============================================================================

/// Response to query request - mirrors OrchestrationResult
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum QueryResponse {
    /// Query completed successfully
    Success {
        sql: String,
        plan: IntermediatePlan,
        #[serde(skip_serializing_if = "Option::is_none")]
        rationale: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        assumptions: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        trace: Option<PlannerTrace>,
    },
    /// Pending user confirmation for proposed changes
    PendingConfirmation {
        session_id: String,
        draft: ReportSpecDraft,
        diffs: Vec<SpecDiff>,
        attempt: usize,
    },
    /// Compilation failed with diagnostics
    CompilationFailed {
        diagnostics: CompilerDiagnostics,
        #[serde(skip_serializing_if = "Option::is_none")]
        draft: Option<ReportSpecDraft>,
    },
    /// Planner failed to generate suggestion
    PlannerFailed { error: PlannerErrorResponse },
    /// Retry limit exceeded
    RetryLimitExceeded {
        diagnostics: CompilerDiagnostics,
        draft: ReportSpecDraft,
        attempts: usize,
    },
}

/// Serializable planner error information
#[derive(Debug, Serialize)]
pub struct PlannerErrorResponse {
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub help: Vec<String>,
}

impl From<PlannerError> for PlannerErrorResponse {
    fn from(error: PlannerError) -> Self {
        let error_str = error.to_string();
        let error_type = match error {
            PlannerError::Unimplemented => "unimplemented",
            PlannerError::InvalidPrompt(_) => "invalid_prompt",
            PlannerError::InternalError(_) => "internal_error",
        };

        Self {
            error_type: error_type.to_string(),
            message: error_str,
            retry_after: None,
            help: vec![],
        }
    }
}

/// Response to confirmation request
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConfirmResponse {
    /// Confirmation processed, query completed
    Success {
        sql: String,
        plan: IntermediatePlan,
        #[serde(skip_serializing_if = "Option::is_none")]
        rationale: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        assumptions: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        trace: Option<PlannerTrace>,
    },
    /// User rejected, session closed
    Rejected,
    /// Modification requested, awaiting new draft
    PendingConfirmation {
        session_id: String,
        draft: ReportSpecDraft,
        diffs: Vec<SpecDiff>,
        attempt: usize,
    },
    /// Compilation failed after modification
    CompilationFailed {
        diagnostics: CompilerDiagnostics,
        #[serde(skip_serializing_if = "Option::is_none")]
        draft: Option<ReportSpecDraft>,
    },
    /// Retry limit exceeded
    RetryLimitExceeded {
        diagnostics: CompilerDiagnostics,
        draft: ReportSpecDraft,
        attempts: usize,
    },
    /// Session not found
    SessionNotFound { session_id: String },
}

// ============================================================================
// Error Response
// ============================================================================

/// Generic error response for API errors
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            details: None,
        }
    }

    pub fn with_details(error: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            details: Some(details.into()),
        }
    }
}
