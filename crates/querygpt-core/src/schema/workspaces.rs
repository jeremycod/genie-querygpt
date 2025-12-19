use std::collections::HashMap;
use crate::schema::field_catalog::{FieldDef, FieldType, WorkspaceSchema};

// TODO: Hardcoding this for now.
// Later, build WorkspaceSchema from Schema Cards JSON
pub fn campaigns_offers_schema() -> WorkspaceSchema {
    let mut fields = HashMap::new();

    // Direct/selectable/sortable fields
    fields.insert("partnership_id".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: true });
    fields.insert("campaign_id".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: true });
    fields.insert("campaign_name".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: true });
    fields.insert("offer_id".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: true });
    fields.insert("offer_name".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: true });
    fields.insert("workflow_status".into(), FieldDef { field_type: FieldType::Enum, selectable: true, filterable: true, sortable: true });
    fields.insert("countries".into(), FieldDef { field_type: FieldType::StringArray, selectable: true, filterable: true, sortable: false });

    // Derived/selectable fields
    fields.insert("package_id".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: false });
    fields.insert("expired_or_live_status".into(), FieldDef { field_type: FieldType::Enum, selectable: true, filterable: false, sortable: true });
    fields.insert("products_csv".into(), FieldDef { field_type: FieldType::String, selectable: true, filterable: false, sortable: false });

    // Filter-only field
    fields.insert("promo_type".into(), FieldDef { field_type: FieldType::Enum, selectable: false, filterable: true, sortable: false });

    WorkspaceSchema {
        workspace: "campaigns_offers".into(),
        fields,
    }
}
