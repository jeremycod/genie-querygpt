use clap::{Parser, Subcommand};
use querygpt_core::planner::fixture_planner::FixturePlanner;
use querygpt_core::planner::openai_client::OpenAIClient;

#[derive(Parser)]
#[command(name = "querygpt")]
#[command(about = "QueryGPT - Natural language to SQL")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate SQL from natural language prompt
    Plan {
        /// Natural language prompt
        #[arg(long)]
        prompt: String,

        /// Auto-approve without confirmation
        #[arg(long)]
        yes: bool,

        /// Maximum retry attempts
        #[arg(long, default_value = "3")]
        max_attempts: usize,

        /// Show detailed explanation
        #[arg(long)]
        explain: bool,

        /// Use OpenAI instead of fixtures (requires OPENAI_API_KEY)
        #[arg(long)]
        openai: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Plan {
            prompt,
            yes,
            max_attempts,
            explain,
            openai,
        } => handle_plan_command(prompt, yes, max_attempts, explain, openai).await,
    }
}

/// Extract retry_after seconds from error message
fn extract_retry_after(error_str: &str) -> Option<u64> {
    error_str
        .split("try again in ")
        .nth(1)
        .and_then(|s| s.split('s').next())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Build a FixturePlanner with common test cases
fn build_fixture_planner() -> FixturePlanner {
    use querygpt_core::dsl::report_spec::{
        Filter, FilterOp, Mode, OrderBy, ReportSpec, SelectItem, SortDir,
    };
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
                field: "campaign_deleted".to_string(),
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
                field: "offer_deleted".to_string(),
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
    use querygpt_core::dsl::report_spec::{
        Filter, FilterOp, Mode, OrderBy, ReportSpec, SelectItem, SortDir,
    };
    use querygpt_core::planner::schema_summary::ExamplePair;
    use serde_json::json;

    vec![
        // Example 1: Simple select with two fields
        ExamplePair {
            prompt: "show all offers".to_string(),
            description: "Select basic fields from offers_latest".to_string(),
            spec: ReportSpec {
                version: 1,
                workspace: "campaigns_offers".to_string(),
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
    ]
}

async fn handle_plan_command(
    prompt: String,
    yes: bool,
    max_attempts: usize,
    _explain: bool,
    openai: bool,
) -> anyhow::Result<()> {
    use querygpt_core::planner::confirmation::{AutoApproveConfirmation, InteractiveConfirmation};
    use querygpt_core::planner::llm_planner::LlmPlanner;
    use querygpt_core::planner::orchestration::Orchestrator;
    use querygpt_core::planner::planner::PlannerContext;
    use querygpt_core::schema::registry::SchemaRegistry;
    use querygpt_core::sql::render::render_sql;

    // Step 1: Load schema registry
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(|p| p.parent())
        .expect("resolve repo root");
    let index_path = repo_root.join("config/workspaces/campaigns_offers.index.json");

    let registry = SchemaRegistry::load(index_path.to_str().unwrap())
        .map_err(|e| anyhow::anyhow!("Failed to load schema registry: {}", e))?;

    // Step 2: Build planner context with full schema
    let schema_summary =
        querygpt_core::planner::schema_summary::SchemaSummary::from_registry(&registry);

    // Extract field and table names for backward compatibility
    let available_fields = schema_summary.get_all_fields();
    let available_tables = schema_summary.get_all_tables();

    // Add example queries to guide the LLM
    let examples = build_example_queries();

    let context = PlannerContext {
        workspace: "campaigns_offers".to_string(),
        schema_summary,
        report_spec_schema: None, // Not needed for LLM planner
        examples,
        constraints: querygpt_core::planner::schema_summary::PlannerConstraints::default(),
        available_fields,
        available_tables,
    };

    // Step 3: Run orchestration based on planner type and confirmation mode
    let result = if openai {
        println!("🤖 Using OpenAI for planning");
        let client = OpenAIClient::from_env().map_err(|e| {
            anyhow::anyhow!(
                "Failed to initialize OpenAI client: {}\n💡 Set OPENAI_API_KEY environment variable",
                e
            )
        })?;
        let planner = LlmPlanner::new(Box::new(client), "gpt-3.5-turbo".to_string());

        if yes {
            let orchestrator =
                Orchestrator::new(planner, AutoApproveConfirmation).with_max_retries(max_attempts);
            orchestrator
                .suggest_and_compile(&registry, &prompt, context)
                .await
        } else {
            let orchestrator =
                Orchestrator::new(planner, InteractiveConfirmation).with_max_retries(max_attempts);
            orchestrator
                .suggest_and_compile(&registry, &prompt, context)
                .await
        }
    } else {
        println!("🧪 Using fixture planner for testing");
        let planner = build_fixture_planner();

        if yes {
            let orchestrator =
                Orchestrator::new(planner, AutoApproveConfirmation).with_max_retries(max_attempts);
            orchestrator
                .suggest_and_compile(&registry, &prompt, context)
                .await
        } else {
            let orchestrator =
                Orchestrator::new(planner, InteractiveConfirmation).with_max_retries(max_attempts);
            orchestrator
                .suggest_and_compile(&registry, &prompt, context)
                .await
        }
    };

    // Step 5: Handle result and display output
    match result {
        querygpt_core::planner::orchestration::OrchestrationResult::Success { plan, .. } => {
            // Render SQL
            let sql = render_sql(&plan)?;

            println!("\n✅ Success! Generated SQL:");
            println!("{}", sql);
            Ok(())
        }
        querygpt_core::planner::orchestration::OrchestrationResult::CompilationFailed {
            diagnostics,
            ..
        } => {
            println!("\n❌ Compilation failed:");
            println!("{:#?}", diagnostics);
            Err(anyhow::anyhow!("Compilation failed"))
        }
        querygpt_core::planner::orchestration::OrchestrationResult::PlannerFailed { error } => {
            println!("\n❌ Planner failed: {}", error);

            // Provide context-specific help based on error type
            let error_str = error.to_string();
            if error_str.contains("timeout") {
                println!("\n💡 Troubleshooting timeout:");
                println!("   • Check your internet connection");
                println!("   • OpenAI API might be experiencing high load");
                println!("   • Try again in a few moments");
            } else if error_str.contains("rate limit") {
                println!("\n💡 Rate limit exceeded:");
                println!("   • Wait a few minutes before trying again");
                println!("   • Consider upgrading your OpenAI API plan");
                if let Some(retry_after) = extract_retry_after(&error_str) {
                    println!("   • Retry after {} seconds", retry_after);
                }
            } else if error_str.contains("authentication") || error_str.contains("API key") {
                println!("\n💡 Authentication failed:");
                println!("   • Check that OPENAI_API_KEY is set correctly");
                println!("   • Verify your API key at https://platform.openai.com/api-keys");
                println!("   • Ensure your API key has not expired");
            } else if error_str.contains("network") || error_str.contains("connect") {
                println!("\n💡 Network error:");
                println!("   • Check your internet connection");
                println!("   • Verify you can reach api.openai.com");
                println!("   • Check if a firewall is blocking the request");
            } else {
                println!("\n💡 Available test prompts:");
                println!("   • show all campaigns");
                println!("   • show all offers");
                println!("   • show active campaigns");
                println!("   • show active offers");
                println!("   • show campaigns ordered by name");
            }
            Err(anyhow::anyhow!("Planner failed: {}", error))
        }
        querygpt_core::planner::orchestration::OrchestrationResult::UserRejected { .. } => {
            println!("\n❌ User rejected the changes");
            Err(anyhow::anyhow!("User rejected"))
        }
        querygpt_core::planner::orchestration::OrchestrationResult::RetryLimitExceeded {
            diagnostics,
            attempts,
            ..
        } => {
            println!("\n❌ Retry limit exceeded after {} attempts", attempts);
            println!("{:#?}", diagnostics);
            Err(anyhow::anyhow!("Retry limit exceeded"))
        }
    }
}
