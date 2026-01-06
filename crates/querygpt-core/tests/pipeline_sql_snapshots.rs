use insta::assert_snapshot;
use querygpt_core::dsl::compile::compile_report_spec;

use querygpt_core::dsl::report_spec::{PaginationSpec, ReportSpec};
use querygpt_core::schema::registry::SchemaRegistry;
use querygpt_core::sql::render::render_sql;

fn test_registry() -> SchemaRegistry {
    // Change to repo root directory for schema loading
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(|p| p.parent())
        .expect("resolve repo root from CARGO_MANIFEST_DIR");

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&repo_root).expect("change to repo root");

    let result = SchemaRegistry::load("config/workspaces/campaigns_offers.index.json")
        .expect("load SchemaRegistry");

    std::env::set_current_dir(original_dir).expect("restore original directory");
    result
}

fn load_spec_json(name: &str) -> ReportSpec {
    // keep fixtures in tests/fixtures/report_specs/
    let raw = match name {
        "base" => include_str!("fixtures/report_specs/campaigns_offers_prepaid_apac.json"),
        _ => panic!("unknown fixture name: {name}"),
    };

    serde_json::from_str(raw).expect("parse ReportSpec")
}

fn compile_and_render(spec: ReportSpec) -> String {
    let reg = test_registry();
    let plan = compile_report_spec(&reg, &spec).expect("compile failed");
    render_sql(&plan).expect("render failed")
}

#[test]
fn pipeline_sql_prepaid_apac_export() {
    let spec = load_spec_json("base");
    let sql = compile_and_render(spec);
    assert_snapshot!("pipeline_sql__prepaid_apac_export", sql);
}

#[test]
fn pipeline_sql_prepaid_apac_export_pagination() {
    let mut spec = load_spec_json("base");
    spec.pagination = Some(PaginationSpec {
        limit: Some(100),
        offset: Some(200),
    });

    let sql = compile_and_render(spec);
    assert_snapshot!("pipeline_sql__prepaid_apac_export_pagination", sql);
}
