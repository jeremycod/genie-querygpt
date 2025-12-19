use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub workspace: String,
    pub task: String,
    pub filter_hints: Vec<String>,
}

/// Stub: classify query to workspace/task.
pub fn classify(_user_prompt: &str) -> IntentResult {
    IntentResult {
        workspace: "campaigns_offers".to_string(),
        task: "export".to_string(),
        filter_hints: vec![],
    }
}
