use clap::{Parser, Subcommand};
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

    // Add example queries to guide the LLM (using shared module)
    let examples = querygpt_core::examples::build_example_queries();

    let context = PlannerContext {
        workspace: "campaigns_offers".to_string(),
        schema_summary,
        report_spec_schema: None, // Not needed for LLM planner
        examples,
        constraints: querygpt_core::planner::schema_summary::PlannerConstraints::default(),
        available_fields,
        available_tables,
        current_date: None,
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
        // Using shared fixtures module
        let planner = querygpt_core::fixtures::build_fixture_planner();

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
