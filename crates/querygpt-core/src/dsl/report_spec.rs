use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportSpec {
    pub version: u32,
    pub workspace: String,
    pub select: Vec<SelectItem>,
    #[serde(default)]
    pub filters: Vec<Filter>,
    #[serde(default)]
    pub order_by: Vec<OrderBy>,
    #[serde(default = "default_mode")]
    pub mode: Mode,
    pub pagination: Option<PaginationSpec>,
}

#[derive(Debug, Deserialize, Clone, Serialize, PartialEq, Eq)]
pub struct PaginationSpec {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn default_mode() -> Mode {
    Mode::Preview
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectItem {
    pub field: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderBy {
    pub field: String,
    pub dir: SortDir,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    In,
    Overlaps,
    Gte,
    Lte,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Preview,
    Export,
}

/// Makes the spec stable for snapshot tests and caching.
/// - preserves `select` order (important for exports)
/// - sorts filters deterministically
pub fn normalize(mut spec: ReportSpec) -> ReportSpec {
    spec.filters.sort_by(|a, b| {
        (a.field.as_str(), &a.op as *const _ as usize)
            .cmp(&(b.field.as_str(), &b.op as *const _ as usize))
    });
    spec
}
