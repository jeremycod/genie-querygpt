use serde::Serialize;
use thiserror::Error;
use crate::dsl::validate::SpecError;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("invalid limit {value}")]
    InvalidLimit { value: i64 },
    #[error("invalid offset {value}")]
    InvalidOffset { value: i64 },
}

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error(transparent)]
    Spec(#[from] SpecError),

    #[error(transparent)]
    Pagination(#[from] CompileError),

    #[error("unknown field {field} in {context}")]
    UnknownField {
        field: String,
        context: &'static str
    },

    #[error("invalid join: {reason}")]
    InvalidJoin { reason: String },

    #[error("invalid projection for field '{field}'")]
    InvalidProjection { field: String },

    #[error("invalid filter for field '{field}'")]
    InvalidFilter { field: String },

    #[error("invalid ordering for field '{field}'")]
    InvalidOrderBy { field: String },

    #[error("schema mismatch: expected {expected}, found {found}")]
    SchemaMismatch { expected: String, found: String },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnknownField,
    InvalidJoin,
    PaginationOutOfRange,
    SchemaMismatch,
    InvalidProjection,
    InvalidFilter,
    InvalidOrderBy,
    WorkspaceNotFound,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompilerDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompileDiagnostics {
    pub diagnostics: Vec<CompilerDiagnostic>,
}

impl CompileDiagnostics {
    pub fn single(diagnostic: CompilerDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    pub fn schema_mismatch(expected: String, found: String) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::SchemaMismatch,
            message: format!(
                "schema registry workspace '{expected}' does not match spec workspace '{found}'"
            ),
            field: None,
            context: None,
            detail: Some(serde_json::json!({ "expected": expected, "found": found })),
        })
    }

    pub fn invalid_join(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::InvalidJoin,
            message: format!("invalid join: {reason}"),
            field: None,
            context: None,
            detail: Some(serde_json::json!({ "reason": reason })),
        })
    }

    pub fn pagination_out_of_range(field: &'static str, value: i64) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::PaginationOutOfRange,
            message: format!("pagination value for '{field}' must be non-negative"),
            field: Some(field.to_string()),
            context: None,
            detail: Some(serde_json::json!({ "value": value })),
        })
    }

    pub fn unknown_field(field: String, context: &'static str) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::UnknownField,
            message: format!("unknown field '{field}' in {context}"),
            field: Some(field),
            context: Some(context.to_string()),
            detail: None,
        })
    }

    pub fn invalid_projection(field: String, message: impl Into<String>) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::InvalidProjection,
            message: message.into(),
            field: Some(field),
            context: Some("select".to_string()),
            detail: None,
        })
    }

    pub fn invalid_filter(field: String, message: impl Into<String>) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::InvalidFilter,
            message: message.into(),
            field: Some(field),
            context: Some("filters".to_string()),
            detail: None,
        })
    }

    pub fn invalid_order(field: String, message: impl Into<String>) -> Self {
        Self::single(CompilerDiagnostic {
            code: DiagnosticCode::InvalidOrderBy,
            message: message.into(),
            field: Some(field),
            context: Some("order_by".to_string()),
            detail: None,
        })
    }
}

impl From<CompileError> for CompileDiagnostics {
    fn from(value: CompileError) -> Self {
        match value {
            CompileError::InvalidLimit { value } => Self::pagination_out_of_range("limit", value),
            CompileError::InvalidOffset { value } => Self::pagination_out_of_range("offset", value),
        }
    }
}

impl From<SpecError> for CompileDiagnostics {
    fn from(value: SpecError) -> Self {
        match value {
            SpecError::UnknownField { field, context } => Self::unknown_field(field, context),
            SpecError::NotSelectable { field } => Self::invalid_projection(
                field.clone(),
                format!("field '{field}' is not selectable"),
            ),
            SpecError::NotFilterable { field } => {
                Self::invalid_filter(field.clone(), format!("field '{field}' is not filterable"))
            }
            SpecError::NotSortable { field } => {
                Self::invalid_order(field.clone(), format!("field '{field}' is not sortable"))
            }
            SpecError::InvalidOperator {
                field,
                op,
                field_type,
            } => Self::invalid_filter(
                field.clone(),
                format!("invalid operator '{op:?}' for type '{field_type:?}'"),
            ),
            SpecError::InvalidValue { field, reason } => {
                Self::invalid_filter(field.clone(), format!("invalid value: {reason}"))
            }
            SpecError::ExportSelectEmpty => Self::invalid_projection(
                "<select>".to_string(),
                "export mode requires at least 1 select field",
            ),
            SpecError::WorkspaceNotFound(workspace) => {
                CompileDiagnostics::single(CompilerDiagnostic {
                    code: DiagnosticCode::WorkspaceNotFound,
                    message: format!("workspace '{workspace}' was not found"),
                    field: None,
                    context: None,
                    detail: Some(serde_json::json!({ "workspace": workspace })),
                })
            }
        }
    }
}
impl From<CompilerError> for CompileDiagnostics {
    fn from(value: CompilerError) -> Self {
        match value {
            CompilerError::Spec(e) => CompileDiagnostics::from(e),
            CompilerError::Pagination(e) => CompileDiagnostics::from(e),
            CompilerError::SchemaMismatch { expected, found } => {
                CompileDiagnostics::schema_mismatch(expected, found)
            }
            CompilerError::InvalidJoin { reason } => CompileDiagnostics::invalid_join(reason),
            CompilerError::UnknownField { field, context } => {
                CompileDiagnostics::unknown_field(field, context)
            }
            CompilerError::InvalidProjection { field } => CompileDiagnostics::invalid_projection(
                field.clone(),
                format!("invalid projection for field '{field}'"),
            ),
            CompilerError::InvalidFilter { field } => CompileDiagnostics::invalid_filter(
                field.clone(),
                format!("invalid filter on field '{field}'"),
            ),
            CompilerError::InvalidOrderBy { field } => CompileDiagnostics::invalid_order(
                field.clone(),
                format!("invalid ordering on field '{field}'"),
            ),
        }
    }
}
