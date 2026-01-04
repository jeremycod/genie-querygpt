use crate::dsl::plan::IntermediatePlan;
use crate::dsl::report_spec::ReportSpec;

#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("planner functionality is not implement yet")]
    Unimplemented,

    #[error("invalid planning prompt: {0}")]
    InvalidPrompt(String),

}
pub trait Planner {
    fn suggest_report_spec(&self, prompt: &str) -> Result<ReportSpec, PlannerError>;
    fn suggest_plan(&self, prompt: &str) -> Result<IntermediatePlan, PlannerError>;

}

#[derive(Debug, Default, Clone, Copy)]
pub struct StubPlanner;

impl Planner for StubPlanner {
    fn suggest_report_spec(&self, prompt: &str) -> Result<ReportSpec, PlannerError> {
        let _ = prompt;
        Err(PlannerError::Unimplemented)
    }

    fn suggest_plan(&self, prompt: &str) -> Result<IntermediatePlan, PlannerError> {
        let _ = prompt;
        Err(PlannerError::Unimplemented)
    }
}
