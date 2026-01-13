//! Example queries for LLM guidance.
//!
//! This module provides production-quality example queries that guide
//! the LLM in generating correct ReportSpec structures. These examples
//! demonstrate proper usage of filters, operators, ordering, and field selection.

use crate::dsl::report_spec::{Filter, FilterOp, Mode, OrderBy, ReportSpec, SelectItem, SortDir};
use crate::planner::schema_summary::ExamplePair;
use serde_json::json;

/// Build comprehensive example queries for LLM context.
///
/// These 10 examples cover:
/// - Basic field selection
/// - Boolean filtering
/// - Date comparisons
/// - Array overlap operations
/// - Multiple combined filters
/// - Ordering patterns
/// - Region-to-country expansion
/// - IN operator usage
/// - NULL checking with eq operator
pub fn build_example_queries() -> Vec<ExamplePair> {
    vec![
        // Example 1: Simple select with two fields
        ExamplePair {
            prompt: "show all offers".to_string(),
            description: "Select basic fields from offers_latest".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 2: With filter on boolean field
        ExamplePair {
            prompt: "show active offers".to_string(),
            description: "Filter offers where deleted is false".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "status".to_string(),
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
        },
        // Example 3: With ORDER BY
        ExamplePair {
            prompt: "list campaigns ordered by name".to_string(),
            description: "Order results by name ascending".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![],
                order_by: vec![OrderBy {
                    field: "name".to_string(),
                    dir: SortDir::Asc,
                }],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 4: Date comparison filter
        ExamplePair {
            prompt: "show offers starting after January 1 2024".to_string(),
            description: "Filter by date using gte operator".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "start_date".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![Filter {
                    field: "start_date".to_string(),
                    op: FilterOp::Gte,
                    value: json!("2024-01-01"),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 5: Array filtering with overlaps operator
        ExamplePair {
            prompt: "show offers available in US".to_string(),
            description: "Filter array field using overlaps operator for geographic queries"
                .to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "countries".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![Filter {
                    field: "countries".to_string(),
                    op: FilterOp::Overlaps,
                    value: json!(["US"]),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 6: Multiple filters combined
        ExamplePair {
            prompt: "show active offers starting in 2024".to_string(),
            description: "Combine multiple filters - boolean and date".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "status".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "start_date".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![
                    Filter {
                        field: "deleted".to_string(),
                        op: FilterOp::Eq,
                        value: json!(false),
                    },
                    Filter {
                        field: "start_date".to_string(),
                        op: FilterOp::Gte,
                        value: json!("2024-01-01"),
                    },
                ],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 7: ORDER BY with field properly included in SELECT
        ExamplePair {
            prompt: "list offers ordered by start date".to_string(),
            description: "Order by date field - note start_date is in SELECT".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "start_date".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![],
                order_by: vec![OrderBy {
                    field: "start_date".to_string(),
                    dir: SortDir::Asc,
                }],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 8: Region query with country code expansion
        ExamplePair {
            prompt: "show offers in APAC countries".to_string(),
            description: "Region names must be expanded to country codes - APAC becomes full Asia-Pacific country list".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "countries".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "status".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![Filter {
                    field: "countries".to_string(),
                    op: FilterOp::Overlaps,
                    value: json!(["AF","AU","BD","BT","BN","KH","CN","HK","IN","ID","JP","KI","KP","KR","LA","MY","MV","MN","MM","NP","NZ","PK","PG","PH","SG","SB","LK","TW","TH","TL","VU","VN"]),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 9: Filter with IN operator
        ExamplePair {
            prompt: "show offers with status active or pending".to_string(),
            description: "Use IN operator for multiple value matching".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "status".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![Filter {
                    field: "status".to_string(),
                    op: FilterOp::In,
                    value: json!(["active", "pending"]),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
        // Example 10: NULL check with eq operator
        ExamplePair {
            prompt: "show offers that have not expired yet".to_string(),
            description:
                "NULL checks use 'eq' operator with null value - NEVER use 'isnull' operator"
                    .to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
                primary_entity: None,
                select: vec![
                    SelectItem {
                        field: "id".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "name".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "endDate".to_string(),
                        alias: None,
                    },
                    SelectItem {
                        field: "status".to_string(),
                        alias: None,
                    },
                ],
                filters: vec![Filter {
                    field: "endDate".to_string(),
                    op: FilterOp::Eq,
                    value: json!(null),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
        },
    ]
}
