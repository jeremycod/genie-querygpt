use clap::{Parser, Subcommand};
use querygpt_core::planner::orchestration::{Orchestrator, OrchestrationResult};
use querygpt_core::planner::planner::PlannerContext;
use querygpt_core::dsl::report_spec::{ReportSpec, SelectItem, Mode};
use querygpt_core::planner::fixture_planner::FixturePlanner;
use querygpt_core::planner::confirmation::{InteractiveConfirmation, AutoApproveConfirmation};
use querygpt_core::planner::trace::FlowLogger;
use querygpt_core::schema::registry::SchemaRegistry;
use querygpt_core::sql::render::render_sql;

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
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Plan { prompt, yes, max_attempts, explain } => {
            handle_plan_command(prompt, yes, max_attempts, explain)
        }
    }
}

fn handle_plan_command(
    prompt: String,
    yes: bool,
    max_attempts: usize,
    _explain: bool,
) -> anyhow::Result<()> {
    // Load schema registry
    let registry = SchemaRegistry::load("config/workspaces/campaigns_offers.index.json")
        .map_err(|e| anyhow::anyhow!("Failed to load schema: {}", e))?;

    // Create planner context
    let context = PlannerContext::simple(
        "campaigns_offers".to_string(),
        vec!["campaign_id".to_string(), "offer_id".to_string()],
        vec!["campaigns_latest".to_string(), "offers_latest".to_string()],
    );

    // Create orchestrator with fixture planner for now
    let mut planner = FixturePlanner::new();
    
    // Add test fixtures
    planner.add_fixture(
        "show me all campaigns".to_string(),
        ReportSpec {
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
        },
    );
    
    planner.add_fixture(
        "export all campaigns".to_string(),
        ReportSpec {
            version: 1,
            workspace: "campaigns_offers".to_string(),
            select: vec![SelectItem {
                field: "campaign_id".to_string(),
                alias: Some("id".to_string()),
            }],
            filters: vec![],
            order_by: vec![],
            mode: Mode::Export,
            pagination: None,
        },
    );
    
    // Choose confirmation based on --yes flag
    let result = if yes {
        let orchestrator = Orchestrator::new(planner, AutoApproveConfirmation)
            .with_max_retries(max_attempts);
        orchestrator.suggest_and_compile(&registry, &prompt, context)
    } else {
        let orchestrator = Orchestrator::new(planner, InteractiveConfirmation)
            .with_max_retries(max_attempts);
        orchestrator.suggest_and_compile(&registry, &prompt, context)
    };
    
    // Execute the flow
    match result {
        OrchestrationResult::Success { plan, trace, .. } => {
            if let Some(trace) = trace {
                println!("✅ Plan generated successfully after {} attempts", trace.attempts);
                if trace.revisions_occurred {
                    println!("🔄 Revisions were made during planning");
                }
            }
            
            FlowLogger::render_sql();
            match render_sql(&plan) {
                Ok(sql) => {
                    println!("\n📋 Generated SQL:");
                    println!("{}", sql);
                }
                Err(e) => {
                    eprintln!("❌ Failed to render SQL: {}", e);
                    return Err(anyhow::anyhow!("SQL rendering failed"));
                }
            }
        }
        OrchestrationResult::PlannerFailed { error } => {
            eprintln!("❌ Planner failed: {}", error);
            return Err(anyhow::anyhow!("Planning failed"));
        }
        OrchestrationResult::CompilationFailed { diagnostics, .. } => {
            eprintln!("❌ Compilation failed:");
            for error in &diagnostics.errors {
                eprintln!("  • {}", error.message);
            }
            return Err(anyhow::anyhow!("Compilation failed"));
        }
        OrchestrationResult::UserRejected { .. } => {
            println!("❌ Changes were rejected");
            return Err(anyhow::anyhow!("User rejected changes"));
        }
        OrchestrationResult::RetryLimitExceeded { attempts, .. } => {
            eprintln!("❌ Retry limit exceeded after {} attempts", attempts);
            return Err(anyhow::anyhow!("Too many retry attempts"));
        }
    }

    Ok(())
}