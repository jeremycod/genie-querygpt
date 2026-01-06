use clap::{Parser, Subcommand};
use querygpt_core::planner::llm::{LlmClient, LlmMessage, LlmRequest, LlmRole};
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

async fn handle_plan_command(
    prompt: String,
    _yes: bool,
    _max_attempts: usize,
    _explain: bool,
    openai: bool,
) -> anyhow::Result<()> {
    if openai {
        return handle_openai_demo(prompt).await;
    }

    println!("❌ Non-OpenAI mode not implemented in this demo");
    println!("💡 Use --openai flag to test OpenAI integration");
    Err(anyhow::anyhow!("Use --openai flag for demo"))
}
async fn handle_openai_demo(prompt: String) -> anyhow::Result<()> {
    println!("🤖 Using OpenAI client for: {}", prompt);

    let client = OpenAIClient::from_env().map_err(|e| {
        anyhow::anyhow!(
            "OpenAI setup failed: {}. Set OPENAI_API_KEY environment variable.",
            e
        )
    })?;

    let request = LlmRequest {
        messages: vec![
            LlmMessage {
                role: LlmRole::System,
                content: r#"You are a ReportSpec generator. Generate valid JSON only.

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- No SQL generation

WORKSPACE: campaigns_offers
AVAILABLE_TABLES: campaigns_latest, offers_latest
AVAILABLE_FIELDS: campaign_id, offer_id

REQUIRED OUTPUT FORMAT:
{
  "report_spec": {
    "version": 1,
    "workspace": "campaigns_offers",
    "select": [{"field": "field_name", "alias": null}],
    "filters": [],
    "order_by": [],
    "mode": "preview",
    "pagination": null
  },
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "explanation"
}

IMPORTANT: Output only valid JSON. No explanations outside the JSON structure."#
                    .to_string(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: format!("Generate a ReportSpec for: {}", prompt),
            },
        ],
        model: "gpt-3.5-turbo".to_string(),
        temperature: 0.1,
        max_tokens: Some(1024),
    };

    println!("📡 Calling OpenAI API...");

    match client.complete(request).await {
        Ok(response) => {
            println!("✅ OpenAI Response received");
            if let Some(usage) = &response.usage {
                println!(
                    "📊 Token usage: {} prompt + {} completion = {} total",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
            println!("\n📋 Generated Response:");
            println!("{}", response.content);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ OpenAI API call failed: {}", e);
            Err(anyhow::anyhow!("OpenAI integration failed"))
        }
    }
}
