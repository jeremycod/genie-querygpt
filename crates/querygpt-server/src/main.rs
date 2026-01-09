mod api_types;
mod confirmation;
mod db;
mod executor;
mod session;

use api_types::{ConfirmRequest, ErrorResponse, QueryRequest, QueryResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use confirmation::ServerConfirmation;
use db::DbPool;
use executor::{ExecutionMode, SqlExecutor};
use querygpt_core::agents::intent;
use querygpt_core::planner::llm_planner::LlmPlanner;
use querygpt_core::planner::openai_client::OpenAIClient;
use querygpt_core::planner::orchestration::{OrchestrationResult, Orchestrator};
use querygpt_core::planner::planner::PlannerContext;
use querygpt_core::planner::schema_summary::SchemaSummary;
use querygpt_core::schema::registry::SchemaRegistry;
use querygpt_core::sql::render::render_sql;
use session::SessionStore;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
struct AppState {
    session_store: SessionStore,
    // In production, this could be a pool of registries
    registry: Arc<SchemaRegistry>,
}

/// Main query endpoint - initiates SQL generation from natural language
async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    // Step 1: Classify intent to determine workspace
    let intent = intent::classify(&req.prompt);

    // Step 2: Build planner context with schema summary and examples
    let schema_summary = SchemaSummary::from_registry(&state.registry);
    let examples = build_example_queries();
    let context =
        PlannerContext::enhanced(intent.workspace.clone(), schema_summary, examples, None);

    // Step 3: Create orchestrator based on planner type
    let confirmation = ServerConfirmation::new(req.auto_approve);
    let result = if let Ok(client) = OpenAIClient::from_env() {
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let planner = LlmPlanner::new(Box::new(client), model);
        let orchestrator =
            Orchestrator::new(planner, confirmation).with_max_retries(req.max_attempts);
        orchestrator
            .suggest_and_compile(&state.registry, &req.prompt, context)
            .await
    } else {
        let planner = build_fixture_planner();
        let orchestrator =
            Orchestrator::new(planner, confirmation).with_max_retries(req.max_attempts);
        orchestrator
            .suggest_and_compile(&state.registry, &req.prompt, context)
            .await
    };

    // Step 6: Convert result to API response
    match result {
        OrchestrationResult::Success {
            plan, draft, trace, ..
        } => {
            let sql = render_sql(&plan)
                .map_err(|e| AppError::RenderError(format!("Failed to render SQL: {}", e)))?;

            Ok(Json(QueryResponse::Success {
                sql,
                plan,
                rationale: draft.as_ref().and_then(|d| d.rationale.clone()),
                assumptions: draft.map(|d| d.assumptions).unwrap_or_default(),
                trace,
                preview_data: None, // TODO: Add preview execution
                pipeline: None,     // TODO: Add pipeline capture
            }))
        }
        OrchestrationResult::CompilationFailed { diagnostics, draft } => {
            Ok(Json(QueryResponse::CompilationFailed {
                diagnostics,
                draft,
            }))
        }
        OrchestrationResult::PlannerFailed { error } => Ok(Json(QueryResponse::PlannerFailed {
            error: error.into(),
        })),
        OrchestrationResult::UserRejected { .. } => {
            // This shouldn't happen in server mode with auto-approve
            // But if it does, treat it as an error
            Err(AppError::UnexpectedState(
                "UserRejected should not occur in server mode".to_string(),
            ))
        }
        OrchestrationResult::RetryLimitExceeded {
            diagnostics,
            draft,
            attempts,
        } => Ok(Json(QueryResponse::RetryLimitExceeded {
            diagnostics,
            draft,
            attempts,
        })),
    }
}

/// Confirmation endpoint - handles user approval/rejection/modification
async fn confirm(
    State(_state): State<AppState>,
    Json(_req): Json<ConfirmRequest>,
) -> Result<Json<api_types::ConfirmResponse>, AppError> {
    // TODO: Implement confirmation flow with session management
    // This requires refactoring the orchestration to be resumable
    Err(AppError::NotImplemented(
        "Confirmation flow not yet implemented".to_string(),
    ))
}

/// Build fixture planner with test cases
fn build_fixture_planner() -> querygpt_core::planner::fixture_planner::FixturePlanner {
    use querygpt_core::dsl::report_spec::{Filter, FilterOp, Mode, ReportSpec, SelectItem};
    use querygpt_core::planner::fixture_planner::FixturePlanner;
    use serde_json::json;

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

    // Fixture 2: Active campaigns
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
                field: "campaign_deleted".to_string(),
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

/// Build example queries to guide the LLM
fn build_example_queries() -> Vec<querygpt_core::planner::schema_summary::ExamplePair> {
    use querygpt_core::dsl::report_spec::{Filter, FilterOp, Mode, ReportSpec, SelectItem};
    use querygpt_core::planner::schema_summary::ExamplePair;
    use serde_json::json;

    vec![
        ExamplePair {
            prompt: "show all offers".to_string(),
            spec: ReportSpec {
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
            description: "Basic query for all offers".to_string(),
        },
        ExamplePair {
            prompt: "show active offers".to_string(),
            spec: ReportSpec {
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
                    field: "offer_deleted".to_string(),
                    op: FilterOp::Eq,
                    value: json!(false),
                }],
                order_by: vec![],
                mode: Mode::Preview,
                pagination: None,
            },
            description: "Filter for active (not deleted) offers using boolean field".to_string(),
        },
    ]
}

/// Application errors
#[derive(Debug)]
enum AppError {
    PlannerInitialization(String),
    RenderError(String),
    UnexpectedState(String),
    NotImplemented(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::PlannerInitialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::RenderError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::UnexpectedState(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
        };

        let error = ErrorResponse::new(message);
        (status, Json(error)).into_response()
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt().init();

    // Initialize session store
    let session_store = SessionStore::with_default_timeout();

    // Load schema registry
    let registry = Arc::new(
        SchemaRegistry::load("config/workspaces/campaigns_offers.index.json")
            .expect("Failed to load schema registry"),
    );

    let state = AppState {
        session_store,
        registry,
    };

    // Build router
    let app = Router::new()
        .route("/query", post(query))
        .route("/confirm", post(confirm))
        .with_state(state);

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind server");

    println!("Server running on {}", bind_addr);
    println!("Endpoints:");
    println!("  POST /query    - Submit natural language query");
    println!("  POST /confirm  - Respond to confirmation request (TODO)");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
