use crate::compile::diagnostics::CompilerDiagnostics;
use serde::Serialize;
use std::time::SystemTime;

/// Trace information for planner operations
#[derive(Debug, Clone, Serialize)]
pub struct PlannerTrace {
    pub model: String,
    pub attempts: usize,
    pub revisions_occurred: bool,
    pub final_status: CompilationStatus,
    #[serde(skip)]
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize)]
pub enum CompilationStatus {
    Success,
    Failed,
    UserRejected,
    RetryLimitExceeded,
    PlannerFailed,
}

impl PlannerTrace {
    pub fn new(model: String) -> Self {
        Self {
            model,
            attempts: 0,
            revisions_occurred: false,
            final_status: CompilationStatus::Failed,
            timestamp: SystemTime::now(),
        }
    }

    pub fn increment_attempt(&mut self) {
        self.attempts += 1;
    }

    pub fn mark_revision(&mut self) {
        self.revisions_occurred = true;
    }

    pub fn set_final_status(&mut self, status: CompilationStatus) {
        self.final_status = status;
    }
}

/// Flow logger for structured trace output
pub struct FlowLogger;

impl FlowLogger {
    pub fn prompt_received(prompt: &str) {
        println!("[flow] prompt received: {}", Self::truncate_prompt(prompt));
    }

    pub fn planner_suggest(attempt: usize) {
        println!("[flow] planner.suggest_report_spec (attempt {})", attempt);
    }

    pub fn planner_revise(attempt: usize) {
        println!("[flow] planner.revise_report_spec (attempt {})", attempt);
    }

    pub fn compiler_result(success: bool) {
        let status = if success { "OK" } else { "ERROR" };
        println!("[flow] compiler.compile_report_spec → {}", status);
    }

    pub fn compiler_diagnostics(diagnostics: &CompilerDiagnostics) {
        if diagnostics.has_errors() {
            println!(
                "[flow] compiler diagnostics: {} errors",
                diagnostics.errors.len()
            );
        }
    }

    pub fn confirm_spec(waiting: bool) {
        let status = if waiting { "waiting" } else { "approved" };
        println!("[flow] confirm spec → {}", status);
    }

    pub fn user_rejected() {
        println!("[flow] user rejected changes");
    }

    pub fn render_sql() {
        println!("[flow] renderer.render_sql → OK");
    }

    pub fn planner_failed(error: &str) {
        println!("[flow] planner failed: {}", error);
    }

    pub fn retry_limit_exceeded(attempts: usize) {
        println!("[flow] retry limit exceeded after {} attempts", attempts);
    }

    fn truncate_prompt(prompt: &str) -> String {
        if prompt.len() > 50 {
            format!("{}...", &prompt[..47])
        } else {
            prompt.to_string()
        }
    }
}
