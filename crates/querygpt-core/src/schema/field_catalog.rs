use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WorkspaceSchema {
    pub workspace: String,
    pub fields: HashMap<String, FieldDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub field_type: FieldType,
    pub selectable: bool,
    pub filterable: bool,
    pub sortable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    StringArray,
    Number,
    Date,
    Enum,
    Bool,
}
