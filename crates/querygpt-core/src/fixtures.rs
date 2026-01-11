//! Shared fixture definitions for testing and demonstrations.
//!
//! This module provides a consistent set of test fixtures used by both
//! querygpt-cli and querygpt-server to ensure identical behavior.

use crate::dsl::report_spec::{Filter, FilterOp, Mode, OrderBy, ReportSpec, SelectItem, SortDir};
use crate::planner::fixture_planner::FixturePlanner;
use serde_json::json;

/// Build the standard fixture planner with 5 test cases.
///
/// These fixtures cover common query patterns:
/// - Simple field selection
/// - Boolean filtering
/// - Ordering
///
/// All fixtures use the "campaigns_offers" workspace.
pub fn build_fixture_planner() -> FixturePlanner {
    let mut planner = FixturePlanner::new();

    // Fixture 1: Simple - show all campaigns
    planner.add_fixture(
        "show all campaigns".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![
                SelectItem {
                    field: "campaign_id".to_string(),
                    alias: None,
                },
                SelectItem {
                    field: "campaign_name".to_string(),
                    alias: None,
                },
            ],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        },
    );

    // Fixture 2: Show all offers (just id and name for now)
    planner.add_fixture(
        "show all offers".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![
                SelectItem {
                    field: "offer_id".to_string(),
                    alias: None,
                },
                SelectItem {
                    field: "offer_name".to_string(),
                    alias: None,
                },
            ],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        },
    );

    // Fixture 3: Active campaigns only (with filter)
    planner.add_fixture(
        "show active campaigns".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![
                SelectItem {
                    field: "campaign_id".to_string(),
                    alias: None,
                },
                SelectItem {
                    field: "campaign_name".to_string(),
                    alias: None,
                },
            ],
            filters: vec![Filter {
                field: "deleted".to_string(),
                op: FilterOp::Eq,
                value: json!(false),
            }],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        },
    );

    // Fixture 4: Campaigns ordered by name
    planner.add_fixture(
        "show campaigns ordered by name".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![
                SelectItem {
                    field: "campaign_id".to_string(),
                    alias: None,
                },
                SelectItem {
                    field: "campaign_name".to_string(),
                    alias: None,
                },
            ],
            filters: vec![],
            order_by: vec![OrderBy {
                field: "campaign_name".to_string(),
                dir: SortDir::Asc,
            }],
            mode: Mode::Preview,
            pagination: None,
        },
    );

    // Fixture 5: Active offers (deleted = false)
    planner.add_fixture(
        "show active offers".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![
                SelectItem {
                    field: "offer_id".to_string(),
                    alias: None,
                },
                SelectItem {
                    field: "offer_name".to_string(),
                    alias: None,
                },
            ],
            filters: vec![Filter {
                field: "deleted".to_string(),
                op: FilterOp::Eq,
                value: json!(false),
            }],
            order_by: vec![],
            mode: Mode::Preview,
            pagination: None,
        },
    );

    planner
}
