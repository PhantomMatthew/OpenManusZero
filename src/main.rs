//! OpenManus - A versatile AI agent framework
//!
//! This is the main entry point for the OpenManus CLI.

use clap::{Parser, Subcommand};
use openmanus::agent::Manus;
use openmanus::llm::{HttpLlmClient, MockLlmClient};
use openmanus::prelude::*;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// OpenManus - A versatile AI agent framework
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// LLM API base URL
    #[arg(
        short,
        long,
        env = "OPENMANUS_ZERO_LLM_BASE_URL",
        default_value = "https://api.openai.com/v1"
    )]
    base_url: String,

    /// LLM API key
    #[arg(short, long, env = "OPENMANUS_ZERO_LLM_API_KEY")]
    api_key: Option<String>,

    /// LLM model to use
    #[arg(short, long, env = "OPENMANUS_ZERO_LLM_MODEL", default_value = "gpt-4")]
    model: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Use mock LLM (for testing)
    #[arg(long)]
    mock: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run with a single prompt
    Run {
        /// The prompt to process
        #[arg(short, long)]
        prompt: String,
    },
    /// Start interactive mode
    Interactive,
    /// Run the flow orchestrator
    Flow {
        /// The task to process
        #[arg(short, long)]
        task: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("OpenManus v{} starting", openmanus::VERSION);

    // Create LLM client
    let llm: Arc<dyn LlmClient> = if args.mock {
        tracing::info!("Using mock LLM client");
        Arc::new(MockLlmClient::new(&args.model))
    } else {
        let api_key = args.api_key.clone().unwrap_or_else(|| {
            tracing::warn!("No API key provided, using mock client");
            "mock-key".to_string()
        });
        Arc::new(HttpLlmClient::new(&args.base_url, &api_key, &args.model))
    };

    match args.command {
        Some(Commands::Run { prompt }) => {
            run_single(&prompt, llm).await?;
        }
        Some(Commands::Interactive) => {
            run_interactive(llm).await?;
        }
        Some(Commands::Flow { task }) => {
            run_flow(&task, llm).await?;
        }
        None => {
            // Default: prompt for input
            let prompt = read_prompt()?;
            if prompt.is_empty() {
                tracing::warn!("Empty prompt provided");
                return Ok(());
            }
            run_single(&prompt, llm).await?;
        }
    }

    Ok(())
}

/// Run with a single prompt
async fn run_single(prompt: &str, llm: Arc<dyn LlmClient>) -> anyhow::Result<()> {
    tracing::info!("Processing prompt: {}", prompt);

    let mut agent = Manus::with_llm(llm);
    let result = agent.run(prompt).await?;

    println!("\n{}", result);

    agent.cleanup().await?;
    Ok(())
}

/// Run in interactive mode
async fn run_interactive(llm: Arc<dyn LlmClient>) -> anyhow::Result<()> {
    println!("OpenManus Interactive Mode (type 'exit' to quit)\n");

    let mut agent = Manus::with_llm(llm);

    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            break;
        }

        match agent.run(input).await {
            Ok(result) => println!("\n{}\n", result),
            Err(e) => tracing::error!("Error: {}", e),
        }
    }

    agent.cleanup().await?;
    println!("Goodbye!");
    Ok(())
}

/// Run with flow orchestration
async fn run_flow(task: &str, llm: Arc<dyn LlmClient>) -> anyhow::Result<()> {
    use openmanus::flow::PlanningFlow;
    use std::collections::HashMap;

    tracing::info!("Running flow for task: {}", task);

    let manus = Manus::with_llm(llm.clone());
    let mut agents = HashMap::new();
    agents.insert("manus".to_string(), Box::new(manus) as Box<dyn Agent>);

    let mut flow = PlanningFlow::new(agents, llm);
    let result = flow.execute(task).await?;

    println!("\n{}", result);

    flow.cleanup().await?;
    Ok(())
}

/// Read a prompt from stdin
fn read_prompt() -> anyhow::Result<String> {
    print!("Enter your prompt: ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
