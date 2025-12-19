use crate::dsl::report_spec::ReportSpec;
use crate::schema::registry::SchemaRegistry;

/// Stub: compile DSL into an intermediate plan (tables, joins, selected fields, predicates).
/// In production, this becomes the deterministic backbone that the LLM must follow.
pub fn compile_report_spec(_reg: &SchemaRegistry, _spec: &ReportSpec) -> anyhow::Result<()> {
    Ok(())
}
