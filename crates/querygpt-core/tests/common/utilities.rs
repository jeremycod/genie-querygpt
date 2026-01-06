use querygpt_core::dsl::report_spec::ReportSpec;
use querygpt_core::schema::registry::SchemaRegistry;
use std::fs;

pub fn load_fixture(name: &str) -> ReportSpec {
    let path = format!("tests/fixtures/report_specs/{}", name);
    let s = fs::read_to_string(path).expect("fixture read");
    serde_json::from_str::<ReportSpec>(&s).expect("fixture parse")
}

pub fn load_schema_registry(name: &str) -> SchemaRegistry {
    // Change to repo root directory for schema loading
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(|p| p.parent())
        .expect("resolve repo root from CARGO_MANIFEST_DIR");

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&repo_root).expect("change to repo root");

    let path = format!("config/workspaces/{}", name);
    let result = SchemaRegistry::load(&path).expect("load schema registry");

    std::env::set_current_dir(original_dir).expect("restore original directory");
    result
}
