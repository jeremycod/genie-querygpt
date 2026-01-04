use querygpt_core::dsl::compile::compile_report_spec;
use querygpt_core::dsl::report_spec::{Mode, PaginationSpec, ReportSpec, SelectItem};
use querygpt_core::schema::cards::{
    ColumnCard, Conventions, EntityCard, EntityKind, JoinEdge, JoinGraph, SchemaCards,
    WorkspaceIndex,
};
use querygpt_core::schema::registry::SchemaRegistry;

use crate::common::{load_fixture, load_schema_registry};

mod common;

#[test]
fn unknown_field_diagnostic() {
    let registry = load_schema_registry("campaigns_offers.index.json");
    let mut spec = load_fixture("campaigns_offers_prepaid_apac.json");
    spec.select = vec![SelectItem {
        field: "nonexistent_field".to_string(),
        alias: None,
    }];

    let diagnostics = compile_report_spec(&registry, &spec).expect_err("expected diagnostics");
    insta::assert_json_snapshot!("unknown_field_diagnostic", diagnostics);
}

#[test]
fn pagination_out_of_range_diagnostic() {
    let registry = load_schema_registry("campaigns_offers.index.json");
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
        pagination: Some(PaginationSpec {
            limit: Some(-5),
            offset: None,
        }),
    };

    let diagnostics =
        compile_report_spec(&registry, &spec).expect_err("expected pagination diagnostics");
    insta::assert_json_snapshot!("pagination_out_of_range_diagnostic", diagnostics);
}

#[test]
fn schema_mismatch_diagnostic() {
    let registry = load_schema_registry("campaigns_offers.index.json");
    let spec = ReportSpec {
        version: 1,
        workspace: "another_workspace".to_string(),
        select: vec![],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let diagnostics =
        compile_report_spec(&registry, &spec).expect_err("expected schema mismatch diagnostics");
    insta::assert_json_snapshot!("schema_mismatch_diagnostic", diagnostics);
}

#[test]
fn invalid_join_diagnostic() {
    let registry = SchemaRegistry {
        index: WorkspaceIndex {
            workspace: "demo".to_string(),
            description: "demo".to_string(),
            schema_cards_path: "".to_string(),
            exemplar_sql_dir: "".to_string(),
            tags: vec![],
            entities: vec!["left".to_string(), "right".to_string()],
        },
        cards: SchemaCards {
            version: "1".to_string(),
            database: "demo".to_string(),
            workspace: "demo".to_string(),
            entities: vec![
                EntityCard {
                    name: "left".to_string(),
                    kind: EntityKind::Table,
                    description: "left".to_string(),
                    primary_key: vec!["left_id".to_string()],
                    columns: vec![
                        ColumnCard {
                            name: "left_id".to_string(),
                            data_type: "text".to_string(),
                            nullable: false,
                            description: "id".to_string(),
                            pii: false,
                        },
                        ColumnCard {
                            name: "shared".to_string(),
                            data_type: "text".to_string(),
                            nullable: false,
                            description: "shared".to_string(),
                            pii: false,
                        },
                    ],
                    json_paths: vec![],
                    common_filters: vec![],
                    tags: vec![],
                },
                EntityCard {
                    name: "right".to_string(),
                    kind: EntityKind::Table,
                    description: "right".to_string(),
                    primary_key: vec!["right_id".to_string()],
                    columns: vec![ColumnCard {
                        name: "right_id".to_string(),
                        data_type: "text".to_string(),
                        nullable: false,
                        description: "id".to_string(),
                        pii: false,
                    }],
                    json_paths: vec![],
                    common_filters: vec![],
                    tags: vec![],
                },
            ],
            join_graph: JoinGraph {
                nodes: vec!["left".to_string(), "right".to_string()],
                edges: vec![JoinEdge {
                    from: "left".to_string(),
                    to: "right".to_string(),
                    join_type: "inner".to_string(),
                    on: vec!["left.left_id right.right_id".to_string()],
                    cardinality: "1:1".to_string(),
                    safe: true,
                    notes: vec![],
                }],
            },
            derived_fields: vec![],
            conventions: Conventions {
                profile_column: "profile".to_string(),
                version_column: "version".to_string(),
                deleted_column: "deleted".to_string(),
                latest_views: vec![],
                notes: vec![],
            },
        },
    };

    let spec = ReportSpec {
        version: 1,
        workspace: "demo".to_string(),
        select: vec![
            SelectItem {
                field: "left_id".to_string(),
                alias: None,
            },
            SelectItem {
                field: "right_id".to_string(),
                alias: None,
            },
        ],
        filters: vec![],
        order_by: vec![],
        mode: Mode::Preview,
        pagination: None,
    };

    let diagnostics = compile_report_spec(&registry, &spec).expect_err("expected join diagnostics");
    insta::assert_json_snapshot!("invalid_join_diagnostic", diagnostics);
}
