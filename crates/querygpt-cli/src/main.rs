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

    // Step 2: Build planner context
    let context = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string(), "campaign_name".to_string()],
        vec!["campaigns_latest".to_string()],
    );

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
            println!("\n💡 Available test prompts:");
            println!("   • show all campaigns");
            println!("   • show all offers");
            println!("   • show active campaigns");
            println!("   • show active offers");
            println!("   • show campaigns ordered by name");
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
