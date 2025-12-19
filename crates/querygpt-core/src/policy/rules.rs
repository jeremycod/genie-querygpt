/// Stub: policy checks (profile isolation, allowed tables, disallow writes).
pub fn enforce_read_only(_sql: &str) -> anyhow::Result<()> { Ok(()) }
