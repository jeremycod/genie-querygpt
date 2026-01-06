use querygpt_core::planner::diff::{diff_report_specs, format_diff_display};
use querygpt_core::dsl::report_spec::{ReportSpec, SelectItem, Filter, OrderBy, FilterOp, SortDir, Mode, PaginationSpec};

#[test]
fn snapshot_diff_no_changes() {
    let spec = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let diffs = diff_report_specs(&spec, &spec);
    let display = format_diff_display(&diffs);

    insta::assert_snapshot!("diff_no_changes", display);
}

#[test]
fn snapshot_diff_added_fields() {
    let original = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let revised = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![
            SelectItem {
                field: "campaign_id".to_string(),
                alias: None,
            },
            SelectItem {
                field: "campaign_name".to_string(),
                alias: Some("name".to_string()),
            },
        ],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let diffs = diff_report_specs(&original, &revised);
    let display = format_diff_display(&diffs);

    insta::assert_snapshot!("diff_added_select_fields", display);
}

#[test]
fn snapshot_diff_added_filters() {
    let original = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let revised = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![Filter {
            field: "region".to_string(),
            op: FilterOp::Eq,
            value: serde_json::Value::String("APAC".to_string()),
        }],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let diffs = diff_report_specs(&original, &revised);
    let display = format_diff_display(&diffs);

    insta::assert_snapshot!("diff_added_filters", display);
}

#[test]
fn snapshot_diff_mode_change() {
    let original = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let revised = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![SelectItem {
            field: "campaign_id".to_string(),
            alias: None,
        }],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Export,
        pagination: None,
    };

    let diffs = diff_report_specs(&original, &revised);
    let display = format_diff_display(&diffs);

    insta::assert_snapshot!("diff_mode_change", display);
}

#[test]
fn snapshot_diff_complex_changes() {
    let original = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![
            SelectItem {
                field: "campaign_id".to_string(),
                alias: None,
            },
            SelectItem {
                field: "old_field".to_string(),
                alias: None,
            },
        ],
        filters: vec![Filter {
            field: "status".to_string(),
            op: FilterOp::Eq,
            value: serde_json::Value::String("active".to_string()),
        }],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let revised = ReportSpec {
        version: 1,
        workspace: "campaigns_offers".to_string(),
        select: vec![
            SelectItem {
                field: "campaign_id".to_string(),
                alias: Some("id".to_string()),
            },
            SelectItem {
                field: "campaign_name".to_string(),
                alias: None,
            },
        ],
        filters: vec![
            Filter {
                field: "status".to_string(),
                op: FilterOp::In,
                value: serde_json::json!(["active", "pending"]),
            },
            Filter {
                field: "region".to_string(),
                op: FilterOp::Eq,
                value: serde_json::Value::String("APAC".to_string()),
            },
        ],
        order_by: vec![OrderBy {
            field: "created_at".to_string(),
            dir: SortDir::Desc,
        }],
        mode: Mode::Export,
        pagination: Some(PaginationSpec {
            limit: Some(100),
            offset: Some(0),
        }),
    };

    let diffs = diff_report_specs(&original, &revised);
    let display = format_diff_display(&diffs);

    insta::assert_snapshot!("diff_complex_changes", display);
}
