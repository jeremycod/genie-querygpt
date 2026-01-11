use anyhow::{anyhow, Result};

use crate::dsl::report_spec::ReportSpec;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type alias for compiler function to reduce complexity
pub type CompilerFn<'a, T> = Box<dyn Fn(&ReportSpec) -> Result<T> + 'a>;

/// Captures the planner-facing state for turning a prompt into a runnable spec.
///
/// The session keeps the original user prompt, the latest LLM-suggested spec,
/// a diff against the user-authored spec, and the latest compiler output. The
/// compiler is invoked every time the suggested spec changes so hosts can rely
/// on fresh planning output.
pub struct PlannerSession<'a, T> {
    user_prompt: String,
    user_spec: ReportSpec,
    suggested_spec: ReportSpec,
    compiler_result: T,
    diff: SpecDiff,
    compiler: CompilerFn<'a, T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecDiff {
    pub user_spec: Value,
    pub suggested_spec: Value,
}

impl SpecDiff {
    pub fn new(user_spec: &ReportSpec, suggested_spec: &ReportSpec) -> Self {
        Self {
            user_spec: serde_json::to_value(user_spec)
                .expect("ReportSpec should serialize to JSON"),
            suggested_spec: serde_json::to_value(suggested_spec)
                .expect("ReportSpec should serialize to JSON"),
        }
    }

    pub fn has_changes(&self) -> bool {
        self.user_spec != self.suggested_spec
    }
}

impl<'a, T> PlannerSession<'a, T> {
    pub fn new<F>(
        user_prompt: impl Into<String>,
        user_spec: ReportSpec,
        suggested_spec: ReportSpec,
        compiler: F,
    ) -> Result<Self>
    where
        F: Fn(&ReportSpec) -> Result<T> + 'a,
    {
        let diff = SpecDiff::new(&user_spec, &suggested_spec);
        let compiler_result = compiler(&suggested_spec)?;

        Ok(Self {
            user_prompt: user_prompt.into(),
            user_spec,
            suggested_spec,
            compiler_result,
            diff,
            compiler: Box::new(compiler),
        })
    }

    pub fn user_prompt(&self) -> &str {
        &self.user_prompt
    }

    pub fn suggested_spec(&self) -> &ReportSpec {
        &self.suggested_spec
    }

    pub fn diff(&self) -> &SpecDiff {
        &self.diff
    }

    pub fn compiler_result(&self) -> &T {
        &self.compiler_result
    }

    /// Replace the suggested spec and immediately re-run the compiler so the
    /// caller always receives fresh planning output.
    pub fn update_suggested_spec(&mut self, suggested_spec: ReportSpec) -> Result<()> {
        self.suggested_spec = suggested_spec;
        self.diff = SpecDiff::new(&self.user_spec, &self.suggested_spec);
        self.compiler_result = (self.compiler)(&self.suggested_spec)?;
        Ok(())
    }

    /// Return the latest runnable spec only when the host explicitly confirms
    /// the action.
    pub fn runnable_spec(&self, confirmed: bool) -> Result<&ReportSpec> {
        if !confirmed {
            return Err(anyhow!(
                "PlannerSession requires explicit confirmation before running a spec"
            ));
        }

        Ok(&self.suggested_spec)
    }
}
