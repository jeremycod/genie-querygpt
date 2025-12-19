use serde::{Deserialize, Serialize};
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSpec {
    pub report: String,
    pub workspace: String,
    pub filters: IndexMap<String, serde_json::Value>,
    pub fields: Vec<String>,
    pub group_by: Vec<String>,
    pub order_by: Vec<String>,
    pub limit: Option<u64>,
}
