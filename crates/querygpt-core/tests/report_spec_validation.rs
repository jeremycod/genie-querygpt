use std::fs;

use querygpt_core::dsl::report_spec::{normalize, ReportSpec};
use querygpt_core::dsl::validate::validate_report_spec;
use querygpt_core::schema::workspaces::campaigns_offers_schema;

fn load_fixture(name: &str) -> ReportSpec {
    let path = format!("tests/fixtures/report_specs/{}", name);
    let s = fs::read_to_string(path).expect("fixture read");
    serde_json::from_str::<ReportSpec>(&s).expect("fixture parse")
}

#[test]
fn parses_and_validates_example_spec() {
    let spec = load_fixture("campaigns_offers_prepaid_apac.json");
    let ws = campaigns_offers_schema();
    validate_report_spec(&spec, Some(&ws)).expect("should validate");
}

#[test]
fn rejects_unknown_field_in_select() {
    let mut spec = load_fixture("campaigns_offers_prepaid_apac.json");
    spec.select.push(querygpt_core::dsl::report_spec::SelectItem {
        field: "does_not_exist".into(),
        alias: None,
    });

    let ws = campaigns_offers_schema();
    let err = validate_report_spec(&spec, Some(&ws)).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown field 'does_not_exist'"));
}

#[test]
fn rejects_invalid_op_overlaps_on_string() {
    let mut spec = load_fixture("campaigns_offers_prepaid_apac.json");
    // Make a bad filter: overlaps on promo_type (enum, filterable but overlaps not valid)
    spec.filters.push(querygpt_core::dsl::report_spec::Filter {
        field: "promo_type".into(),
        op: querygpt_core::dsl::report_spec::FilterOp::Overlaps,
        value: serde_json::json!(["x"]),
    });

    let ws = campaigns_offers_schema();
    let err = validate_report_spec(&spec, Some(&ws)).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("invalid operator"));
}

#[test]
fn snapshot_normalized_spec() {
    let spec = load_fixture("campaigns_offers_prepaid_apac.json");
    let normalized = normalize(spec);
    insta::assert_json_snapshot!(normalized);
}
