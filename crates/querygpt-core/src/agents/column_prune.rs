use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedSchema {
    pub entities: Vec<String>,
    pub columns: Vec<String>,
}

/// Stub: compute minimal columns needed.
pub fn prune(_requested_fields: &[String]) -> PrunedSchema {
    PrunedSchema {
        entities: vec![],
        columns: vec![],
    }
}
