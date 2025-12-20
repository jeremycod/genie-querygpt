use std::fs;
use querygpt_core::dsl::report_spec::ReportSpec;
use querygpt_core::schema::registry::SchemaRegistry;

pub fn load_fixture(name: &str) -> ReportSpec {
    let path = format!("tests/fixtures/report_specs/{}", name);
    let s = fs::read_to_string(path).expect("fixture read");
    serde_json::from_str::<ReportSpec>(&s).expect("fixture parse")
}

pub fn load_schema_registry(name: &str) -> SchemaRegistry {
    let path = format!("../../config/workspaces/{}", name);
    SchemaRegistry::load(&path).expect("load schema registry")
}