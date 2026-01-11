mod api_types;
mod confirmation;
mod db;
mod executor;
mod export;
mod session;

use api_types::{
    ConfirmRequest, ErrorResponse, ExecuteRequest, ExportFormat, ExportRequest, QueryRequest,
    QueryResponse, WorkspaceInfo, WorkspacesResponse,
};
use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use confirmation::ServerConfirmation;
use db::DbPool;
use executor::{ExecutionMode, SqlExecutor};
use querygpt_core::agents::intent::IntentAgent;
use querygpt_core::planner::llm_planner::LlmPlanner;
use querygpt_core::planner::openai_client::OpenAIClient;
use querygpt_core::planner::orchestration::{OrchestrationResult, Orchestrator};
use querygpt_core::planner::planner::PlannerContext;
use querygpt_core::planner::schema_summary::SchemaSummary;
use querygpt_core::schema::registry::WorkspaceRegistry;
use querygpt_core::sql::render::render_sql;
use session::SessionStore;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

/// Shared application state
#[derive(Clone)]
struct AppState {
    session_store: SessionStore,
    /// Workspace registry loaded from configuration
    workspace_registry: Arc<WorkspaceRegistry>,
    /// Intent agent for workspace classification
    intent_agent: Arc<IntentAgent>,
    /// Database connection pool (optional - only if DATABASE_URL is configured)
    db_pool: Option<DbPool>,
}

/// Main query endpoint - initiates SQL generation from natural language
async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    // Step 1: Determine workspace (manual override or auto-classify)
    let workspace = if let Some(ref manual_workspace) = req.workspace {
        // Manual workspace specified
        if !state.workspace_registry.has_workspace(manual_workspace) {
            return Err(AppError::InvalidWorkspace(format!(
                "Workspace '{}' not found. Available workspaces: {}",
                manual_workspace,
                state.workspace_registry.list_workspaces().join(", ")
            )));
        }
        tracing::info!("Using manually specified workspace: {}", manual_workspace);
        manual_workspace.clone()
    } else {
        // Auto-classify workspace from prompt
        let classification = state
            .intent_agent
            .classify_workspace(&req.prompt)
            .await
            .map_err(|e| {
                AppError::WorkspaceClassification(format!("Failed to classify workspace: {}", e))
            })?;

        tracing::info!(
            "Classified workspace: {} (confidence: {:?}, reason: {})",
            classification.workspace,
            classification.confidence,
            classification.reason
        );
        classification.workspace
    };

    // Step 2: Load the workspace schema
    let schema_registry = state
        .workspace_registry
        .load_workspace(&workspace)
        .map_err(|e| {
            AppError::WorkspaceLoad(format!("Failed to load workspace '{}': {}", workspace, e))
        })?;

    // Step 3: Build planner context with schema summary and examples (using shared module)
    let schema_summary = SchemaSummary::from_registry(&schema_registry);
    let examples = querygpt_core::examples::build_example_queries();

    // Get current date for "today" queries
    let current_date = chrono::Local::now().format("%Y-%m-%d").to_string();

    let context = PlannerContext::enhanced(workspace.clone(), schema_summary, examples, None)
        .with_current_date(current_date);

    // Step 4: Create orchestrator based on planner type
    let confirmation = ServerConfirmation::new(req.auto_approve);
    let result = if let Ok(client) = OpenAIClient::from_env() {
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let planner = LlmPlanner::new(Box::new(client), model);
        let orchestrator =
            Orchestrator::new(planner, confirmation).with_max_retries(req.max_attempts);
        orchestrator
            .suggest_and_compile(&schema_registry, &req.prompt, context)
            .await
    } else {
        // Using shared fixtures module
        let planner = querygpt_core::fixtures::build_fixture_planner();
        let orchestrator =
            Orchestrator::new(planner, confirmation).with_max_retries(req.max_attempts);
        orchestrator
            .suggest_and_compile(&schema_registry, &req.prompt, context)
            .await
    };

    // Step 6: Convert result to API response
    match result {
        OrchestrationResult::Success {
            plan, draft, trace, ..
        } => {
            let sql = render_sql(&plan)
                .map_err(|e| AppError::RenderError(format!("Failed to render SQL: {}", e)))?;

            // Debug: Print generated SQL
            eprintln!("[DEBUG] Generated SQL:\n{}", sql);

            // Step 7: Execute preview if requested and database is configured
            let preview_data = if req.execute_preview {
                if let Some(ref db_pool) = state.db_pool {
                    let executor = SqlExecutor::new(db_pool.clone());
                    let mode = ExecutionMode::Preview {
                        limit: req.preview_limit,
                    };

                    match executor.execute(&sql, mode).await {
                        Ok(result) => {
                            tracing::info!(
                                "Preview executed successfully: {} rows in {}ms",
                                result.total_rows,
                                result.execution_time_ms
                            );
                            Some(result)
                        }
                        Err(e) => {
                            tracing::warn!("Preview execution failed: {}", e);
                            // Don't fail the request, just log the error
                            None
                        }
                    }
                } else {
                    tracing::warn!("Preview execution requested but database not configured");
                    None
                }
            } else {
                None
            };

            Ok(Json(QueryResponse::Success {
                sql,
                plan,
                workspace,
                rationale: draft.as_ref().and_then(|d| d.rationale.clone()),
                assumptions: draft.map(|d| d.assumptions).unwrap_or_default(),
                trace,
                preview_data,
                pipeline: None, // TODO: Add pipeline capture in future PR
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

/// Execute SQL directly and return results
///
/// This endpoint executes SQL without going through the query generation pipeline.
/// Useful for:
/// - Refreshing preview data
/// - Re-executing queries with different limits
/// - Testing SQL directly
async fn execute(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<executor::QueryResult>, AppError> {
    // Check if database is configured
    let db_pool = state.db_pool.as_ref().ok_or_else(|| {
        AppError::NotImplemented(
            "Database not configured. Set DATABASE_URL environment variable.".to_string(),
        )
    })?;

    // Create executor
    let executor = SqlExecutor::new(db_pool.clone());

    // Determine execution mode from request
    let mode = match req.mode {
        ExecutionMode::Preview { .. } => ExecutionMode::Preview { limit: req.limit },
        ExecutionMode::Export => ExecutionMode::Export,
    };

    // Execute SQL
    let result = executor.execute(&req.sql, mode).await.map_err(|e| {
        tracing::error!("SQL execution failed: {}", e);
        AppError::ExecutionFailed(format!("Query execution failed: {}", e))
    })?;

    tracing::info!(
        "SQL executed successfully: {} rows in {}ms",
        result.total_rows,
        result.execution_time_ms
    );

    Ok(Json(result))
}

/// List all available workspaces (GET /workspaces)
async fn list_workspaces(State(state): State<AppState>) -> Json<WorkspacesResponse> {
    let workspaces = state
        .workspace_registry
        .all_metadata()
        .iter()
        .map(|metadata| WorkspaceInfo {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            entities: metadata.entities.clone(),
        })
        .collect();

    Json(WorkspacesResponse { workspaces })
}

/// Export query results to CSV or JSON format
///
/// This endpoint executes SQL and returns results as a downloadable file.
/// Supports both CSV and JSON formats with appropriate headers for downloads.
/// Useful for exporting large datasets or generating reports.
async fn export(
    State(state): State<AppState>,
    Json(req): Json<ExportRequest>,
) -> Result<Response, AppError> {
    // Check if database is configured
    let db_pool = state.db_pool.as_ref().ok_or_else(|| {
        AppError::NotImplemented(
            "Database not configured. Set DATABASE_URL environment variable.".to_string(),
        )
    })?;

    // Execute export based on format
    let response = match req.format {
        ExportFormat::Csv => {
            let exporter = export::CsvExporter::new(db_pool.clone());
            exporter
                .export(&req.sql)
                .await
                .map_err(|e| AppError::ExecutionFailed(format!("CSV export failed: {}", e)))?
        }
        ExportFormat::Json => {
            let exporter = export::JsonExporter::new(db_pool.clone());
            exporter
                .export(&req.sql)
                .await
                .map_err(|e| AppError::ExecutionFailed(format!("JSON export failed: {}", e)))?
        }
    };

    tracing::info!(
        "Export completed successfully: format={:?}, session_id={:?}",
        req.format,
        req.session_id
    );

    Ok(response)
}

/// Application errors
#[derive(Debug)]
enum AppError {
    PlannerInitialization(String),
    RenderError(String),
    UnexpectedState(String),
    NotImplemented(String),
    ExecutionFailed(String),
    InvalidWorkspace(String),
    WorkspaceClassification(String),
    WorkspaceLoad(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::PlannerInitialization(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::RenderError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::UnexpectedState(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
            AppError::ExecutionFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InvalidWorkspace(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::WorkspaceClassification(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::WorkspaceLoad(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
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

    // Load workspace registry
    let workspace_registry = Arc::new(
        WorkspaceRegistry::from_directory("config/workspaces")
            .expect("Failed to load workspace registry"),
    );

    tracing::info!(
        "Loaded {} workspaces: {}",
        workspace_registry.list_workspaces().len(),
        workspace_registry.list_workspaces().join(", ")
    );

    // Create IntentAgent for workspace classification
    let llm_client = OpenAIClient::from_env()
        .ok()
        .map(|c| Box::new(c) as Box<dyn querygpt_core::planner::llm::LlmClient>);
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string());
    let intent_agent = Arc::new(IntentAgent::new(
        (*workspace_registry).clone(),
        llm_client,
        model,
    ));

    // Initialize database pool (optional - only if DATABASE_URL is configured)
    let db_pool = match DbPool::from_env() {
        Ok(pool) => {
            tracing::info!("Database connection pool initialized successfully");
            Some(pool)
        }
        Err(e) => {
            tracing::warn!(
                "Database not configured: {}. Preview execution will be unavailable.",
                e
            );
            None
        }
    };

    let state = AppState {
        session_store,
        workspace_registry,
        intent_agent,
        db_pool: db_pool.clone(),
    };

    // Configure CORS for frontend integration
    let cors = CorsLayer::new()
        // Allow requests from Vite dev server
        .allow_origin(
            "http://localhost:5173"
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);

    // Build router
    let app = Router::new()
        .route("/query", post(query))
        .route("/execute", post(execute))
        .route("/export", post(export))
        .route("/confirm", post(confirm))
        .route("/workspaces", get(list_workspaces))
        .layer(cors)
        .with_state(state);

    // Start server
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind server");

    println!("Server running on {}", bind_addr);
    println!(
        "Database: {}",
        if db_pool.is_some() {
            "Connected ✓"
        } else {
            "Not configured (preview execution disabled)"
        }
    );
    println!("CORS: Enabled for http://localhost:5173");
    println!();
    println!("Endpoints:");
    println!("  POST /query    - Submit natural language query");
    println!("                   Supports 'execute_preview' for instant results");
    println!("  POST /execute  - Execute SQL directly (preview or export mode)");
    println!("  POST /export   - Download query results as CSV or JSON");
    println!("  POST /confirm  - Respond to confirmation request (TODO)");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
