use crate::dsl::report_spec::{FilterOp, Mode, ReportSpec};
use crate::schema::field_catalog::{FieldType, WorkspaceSchema};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpecError {
    #[error("workspace '{0}' not found")]
    WorkspaceNotFound(String),

    #[error("unknown field '{field}' in {context}")]
    UnknownField { field: String, context: &'static str },

    #[error("field '{field}' is not selectable")]
    NotSelectable { field: String },

    #[error("field '{field}' is not filterable")]
    NotFilterable { field: String },

    #[error("field '{field}' is not sortable")]
    NotSortable { field: String },

    #[error("invalid operator '{op:?}' for field '{field}' of type '{field_type:?}'")]
    InvalidOperator {
        field: String,
        op: FilterOp,
        field_type: FieldType,
    },

    #[error("invalid value for field '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("export mode requires at least 1 select field")]
    ExportSelectEmpty,
}

pub fn validate_report_spec(spec: &ReportSpec, ws: Option<&WorkspaceSchema>) -> Result<(), SpecError> {
    let ws = ws.ok_or_else(|| SpecError::WorkspaceNotFound(spec.workspace.clone()))?;

    if matches!(spec.mode, Mode::Export) && spec.select.is_empty() {
        return Err(SpecError::ExportSelectEmpty);
    }

    // Validate select fields
    for sel in &spec.select {
        let def = ws
            .fields
            .get(&sel.field)
            .ok_or_else(|| SpecError::UnknownField {
                field: sel.field.clone(),
                context: "select",
            })?;

        if !def.selectable {
            return Err(SpecError::NotSelectable {
                field: sel.field.clone(),
            });
        }
    }

    // Validate filters
    for f in &spec.filters {
        let def = ws.fields.get(&f.field).ok_or_else(|| SpecError::UnknownField {
            field: f.field.clone(),
            context: "filters",
        })?;

        if !def.filterable {
            return Err(SpecError::NotFilterable { field: f.field.clone() });
        }

        validate_filter_op(&f.field, def.field_type, f.op)?;
        validate_filter_value(&f.field, def.field_type, &f.value, f.op)?;
    }

    // Validate order_by
    for ob in &spec.order_by {
        let def = ws.fields.get(&ob.field).ok_or_else(|| SpecError::UnknownField {
            field: ob.field.clone(),
            context: "order_by",
        })?;

        if !def.sortable {
            return Err(SpecError::NotSortable { field: ob.field.clone() });
        }
    }

    Ok(())
}

fn validate_filter_op(field: &str, ty: FieldType, op: FilterOp) -> Result<(), SpecError> {
    use FieldType::*;
    use FilterOp::*;

    let ok = match (ty, op) {
        // eq works for most scalars
        (String | Number | Date | Enum | Bool, Eq) => true,

        // in works for scalar types (value must be array)
        (String | Number | Date | Enum, In) => true,

        // overlaps only for arrays
        (StringArray, Overlaps) => true,

        // comparisons for dates/numbers
        (Date | Number, Gte | Lte) => true,

        _ => false,
    };

    if ok {
        Ok(())
    } else {
        Err(SpecError::InvalidOperator {
            field: field.to_string(),
            op,
            field_type: ty,
        })
    }
}

fn validate_filter_value(field: &str, ty: FieldType, v: &Value, op: FilterOp) -> Result<(), SpecError> {
    use FieldType::*;
    use FilterOp::*;

    let err = |reason: &str| Err(SpecError::InvalidValue { field: field.to_string(), reason: reason.to_string() });

    match (ty, op) {
        (String | Enum, Eq) => v.as_str().map(|_| ()).ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected string".to_string() })?,
        (Bool, Eq) => v.as_bool().map(|_| ()).ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected boolean".to_string() })?,
        (Number, Eq) => v.as_f64().map(|_| ()).ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected number".to_string() })?,
        (Date, Eq) => v.as_str().map(|_| ()).ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected date string".to_string() })?,

        (String | Enum | Number | Date, In) => {
            let arr = v.as_array().ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected array for 'in'".to_string() })?;
            if arr.is_empty() {
                return err("array for 'in' must not be empty");
            }
        }

        (StringArray, Overlaps) => {
            let arr = v.as_array().ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected array for 'overlaps'".to_string() })?;
            if arr.is_empty() {
                return err("array for 'overlaps' must not be empty");
            }
        }

        (Number | Date, Gte | Lte) => {
            // simplest: accept string for Date and number for Number
            match ty {
                Number => { v.as_f64().ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected number".to_string() })?; }
                Date => { v.as_str().ok_or_else(|| SpecError::InvalidValue { field: field.to_string(), reason: "expected date string".to_string() })?; }
                _ => {}
            }
        }

        _ => {} // op/type mismatch already caught
    }

    Ok(())
}
