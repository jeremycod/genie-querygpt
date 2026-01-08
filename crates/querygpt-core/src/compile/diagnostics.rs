use crate::dsl::validate::SpecError;
use serde::Serialize;
use thiserror::Error;

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
        context: &'static str,
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

/// Phase B Diagnostic Codes - Stable and versioned
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnknownField,
    UnknownTable,
    InvalidJoin,
    AmbiguousJoin,
    InvalidPagination,
    SchemaMismatch,
    InvalidFilterValue,
    InvalidProjection,
    InvalidFilter,
    InvalidOrderBy,
    WorkspaceNotFound,
}

/// JSON Pointer span for precise error location
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Span {
    pub pointer: String, // JSON Pointer into ReportSpec
}

/// Phase B Structured Diagnostic - Machine-readable, stable, snapshot-tested
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub spans: Vec<Span>,
    pub details: serde_json::Value,
    pub help: Vec<String>,
}

/// Phase B Compiler Diagnostics - Container for all diagnostics
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompilerDiagnostics {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

impl CompilerDiagnostics {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn error(diagnostic: Diagnostic) -> Self {
        Self {
            errors: vec![diagnostic],
            warnings: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Create unknown field diagnostic
    pub fn unknown_field(field: String, context: &'static str) -> Self {
        let mut help = Vec::new();

        // If field contains dot or SQL qualifier, provide specific guidance
        if field.contains('.') {
            help.push(
                "❌ NEVER use SQL-qualified names like 'o.id', 'c.name', 'table.field'".to_string(),
            );
            help.push(
                "✅ Use ONLY logical field names: 'id', 'name', 'brand', 'status'".to_string(),
            );

            // Try to suggest the fix
            if let Some((_table, field_part)) = field.rsplit_once('.') {
                help.push(format!(
                    "Suggestion: Remove table prefix, use '{}' instead of '{}'",
                    field_part, field
                ));
            }
        } else {
            help.push("Check available fields in the schema".to_string());
            help.push("Verify field name spelling".to_string());
        }

        Self::error(Diagnostic {
            code: DiagnosticCode::UnknownField,
            message: format!("unknown field '{field}' in {context}"),
            spans: vec![Span {
                pointer: format!("/{context}/{}", field.replace('.', "/")),
            }],
            details: serde_json::json!({
                "field": field,
                "context": context
            }),
            help,
        })
    }

    /// Create schema mismatch diagnostic
    pub fn schema_mismatch(expected: String, found: String) -> Self {
        Self::error(Diagnostic {
            code: DiagnosticCode::SchemaMismatch,
            message: format!(
                "schema registry workspace '{expected}' does not match spec workspace '{found}'"
            ),
            spans: vec![Span {
                pointer: "/workspace".to_string(),
            }],
            details: serde_json::json!({
                "expected": expected,
                "found": found
            }),
            help: vec![
                format!("Change workspace to '{expected}'"),
                "Verify the correct schema registry is loaded".to_string(),
            ],
        })
    }

    /// Create invalid join diagnostic
    pub fn invalid_join(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::error(Diagnostic {
            code: DiagnosticCode::InvalidJoin,
            message: format!("invalid join: {reason}"),
            spans: vec![], // TODO: Add specific spans when we have join location info
            details: serde_json::json!({
                "reason": reason
            }),
            help: vec![
                "Check that all referenced tables exist".to_string(),
                "Verify join conditions are valid".to_string(),
            ],
        })
    }

    /// Create pagination out of range diagnostic
    pub fn pagination_out_of_range(field: &'static str, value: i64) -> Self {
        Self::error(Diagnostic {
            code: DiagnosticCode::InvalidPagination,
            message: format!("pagination value for '{field}' must be non-negative"),
            spans: vec![Span {
                pointer: format!("/pagination/{field}"),
            }],
            details: serde_json::json!({
                "field": field,
                "value": value
            }),
            help: vec![
                format!("Set {field} to a non-negative value"),
                "Remove the pagination field if not needed".to_string(),
            ],
        })
    }
}

/// Legacy diagnostics for backward compatibility during transition
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

/// Legacy diagnostics container for backward compatibility
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
            code: DiagnosticCode::InvalidPagination,
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

// Conversion implementations for backward compatibility
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

// Conversion from legacy to new diagnostics
impl From<CompileDiagnostics> for CompilerDiagnostics {
    fn from(legacy: CompileDiagnostics) -> Self {
        let errors = legacy
            .diagnostics
            .into_iter()
            .map(|d| Diagnostic {
                code: d.code,
                message: d.message,
                spans: if let Some(field) = &d.field {
                    vec![Span {
                        pointer: format!(
                            "/{}/{}",
                            d.context.as_deref().unwrap_or("unknown"),
                            field
                        ),
                    }]
                } else {
                    vec![]
                },
                details: d.detail.unwrap_or(serde_json::Value::Null),
                help: vec![], // Legacy diagnostics don't have help
            })
            .collect();

        Self {
            errors,
            warnings: vec![],
        }
    }
}

// Conversion from CompilerError to new CompilerDiagnostics
impl From<CompilerError> for CompilerDiagnostics {
    fn from(value: CompilerError) -> Self {
        match value {
            CompilerError::Spec(e) => {
                // Convert SpecError to legacy first, then to new
                let legacy = CompileDiagnostics::from(e);
                CompilerDiagnostics::from(legacy)
            }
            CompilerError::Pagination(e) => {
                let legacy = CompileDiagnostics::from(e);
                CompilerDiagnostics::from(legacy)
            }
            CompilerError::SchemaMismatch { expected, found } => {
                CompilerDiagnostics::schema_mismatch(expected, found)
            }
            CompilerError::InvalidJoin { reason } => CompilerDiagnostics::invalid_join(reason),
            CompilerError::UnknownField { field, context } => {
                CompilerDiagnostics::unknown_field(field, context)
            }
            CompilerError::InvalidProjection { field } => CompilerDiagnostics::error(Diagnostic {
                code: DiagnosticCode::InvalidProjection,
                message: format!("invalid projection for field '{field}'"),
                spans: vec![Span {
                    pointer: format!("/select/{}", field.replace('.', "/")),
                }],
                details: serde_json::json!({ "field": field }),
                help: vec![
                    "Check that the field is selectable".to_string(),
                    "Verify field exists in the schema".to_string(),
                ],
            }),
            CompilerError::InvalidFilter { field } => CompilerDiagnostics::error(Diagnostic {
                code: DiagnosticCode::InvalidFilter,
                message: format!("invalid filter on field '{field}'"),
                spans: vec![Span {
                    pointer: format!("/filters/{}", field.replace('.', "/")),
                }],
                details: serde_json::json!({ "field": field }),
                help: vec![
                    "Check that the field is filterable".to_string(),
                    "Verify the filter operator is valid for this field type".to_string(),
                ],
            }),
            CompilerError::InvalidOrderBy { field } => CompilerDiagnostics::error(Diagnostic {
                code: DiagnosticCode::InvalidOrderBy,
                message: format!("invalid ordering on field '{field}'"),
                spans: vec![Span {
                    pointer: format!("/order_by/{}", field.replace('.', "/")),
                }],
                details: serde_json::json!({ "field": field }),
                help: vec![
                    "Check that the field is sortable".to_string(),
                    "Verify field exists in the schema".to_string(),
                ],
            }),
        }
    }
}
