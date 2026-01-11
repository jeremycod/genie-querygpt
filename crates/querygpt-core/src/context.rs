//! Helper functions for building PlannerContext.
//!
//! This module provides convenient helpers for constructing PlannerContext
//! instances with common patterns used across CLI and server.

use crate::planner::planner::PlannerContext;
use crate::planner::schema_summary::{PlannerConstraints, SchemaSummary};
use crate::schema::registry::SchemaRegistry;

/// Build a PlannerContext from a SchemaRegistry with examples.
///
/// This helper encapsulates the common pattern of:
/// 1. Creating SchemaSummary from registry
/// 2. Loading example queries
/// 3. Building enhanced context
/// 4. Optionally adding current date
///
/// # Arguments
/// * `registry` - Schema registry to build context from
/// * `workspace` - Workspace name (e.g., "campaigns_offers")
/// * `examples` - Example queries for LLM guidance
/// * `current_date` - Optional current date in YYYY-MM-DD format
/// * `constraints` - Optional planner constraints
pub fn build_planner_context(
    registry: &SchemaRegistry,
    workspace: String,
    examples: Vec<crate::planner::schema_summary::ExamplePair>,
    current_date: Option<String>,
    constraints: Option<PlannerConstraints>,
) -> PlannerContext {
    let schema_summary = SchemaSummary::from_registry(registry);

    let mut context = PlannerContext::enhanced(workspace, schema_summary, examples, constraints);

    if let Some(date) = current_date {
        context = context.with_current_date(date);
    }

    context
}

/// Build a PlannerContext with default workspace ("campaigns_offers").
///
/// Convenience wrapper that uses:
/// - Workspace: "campaigns_offers"
/// - No constraints
/// - No current date (caller should use with_current_date() if needed)
pub fn build_default_context(
    registry: &SchemaRegistry,
    examples: Vec<crate::planner::schema_summary::ExamplePair>,
) -> PlannerContext {
    build_planner_context(
        registry,
        "campaigns_offers".to_string(),
        examples,
        None,
        None,
    )
}
