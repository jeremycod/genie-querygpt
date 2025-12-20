use querygpt_core::dsl::compile::compile_report_spec;

mod common;
use crate::common::{load_fixture, load_schema_registry};




/// Loads a ReportSpec fixture from JSON and compiles it.
/// This test will fail until `compile_report_spec` returns an `IntermediatePlan`.
#[test]
fn compile_prepaid_apac_export() {
    // 1. Load the schema registry for the campaigns_offers workspace.
    //    In your repo, the workspace index JSON (pointing at schema_cards.json)
    //    lives under `config/workspaces/campaigns_offers.index.json`.
    let registry = load_schema_registry("campaigns_offers.index.json");

    // 2. Load the example report spec from the fixture.
    //    This uses the same prepaid APAC report you validated in Ticket #1.
    let spec = load_fixture("campaigns_offers_prepaid_apac.json");

    // 3. Compile the spec into an IntermediatePlan.
    //    At this point, compile_report_spec should not return `Ok(())`,
    //    but rather an actual plan structure.
    let plan = compile_report_spec(&registry, &spec)
        .expect("compile report spec");

    // 4. Use a snapshot assertion to lock down the plan structure.
    //    Once your compiler is implemented, this snapshot will show
    //    all tables, joins, selected fields, filters, and ordering.
    insta::assert_json_snapshot!(plan);
}
