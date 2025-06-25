use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;
use tracing::Level;

mod commands;
mod config;

use commands::*;

/// Solana Watchtower - End-to-end monitoring and alert system for deployed Solana programs
#[derive(Parser)]
#[command(name = "watchtower")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Real-time monitoring and alerting for Solana programs")]
#[command(long_about = None)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the monitoring system
    Start {
        /// Run as background daemon
        #[arg(short, long)]
        daemon: bool,

        /// Dashboard port (overrides config file)
        #[arg(long)]
        dashboard_port: Option<u16>,

        /// Prometheus metrics port
        #[arg(long, default_value = "9090")]
        metrics_port: u16,
    },

    /// Test notification channels
    TestNotifications {
        /// Test specific channel (email, telegram, slack, discord)
        #[arg(short = 't', long)]
        channel: Option<String>,
    },

    /// Validate configuration file
    ValidateConfig,

    /// Manage monitoring rules
    Rules {
        #[command(subcommand)]
        action: RuleAction,
    },

    /// Show system status and statistics
    Status,

    /// Stop running watchtower instance
    Stop,
}

#[derive(Subcommand)]
enum RuleAction {
    /// List available rules
    List,
    /// Show rule information
    Info { rule_name: String },
    /// Test rule with sample data
    Test { rule_name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.debug)?;

    // Print welcome message
    print_banner();

    // Get config path
    let config_path = cli.config.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("watchtower.toml")
    });

    // Execute command
    match cli.command {
        Commands::Start {
            daemon,
            dashboard_port,
            metrics_port,
        } => {
            start_command(config_path, daemon, dashboard_port, metrics_port).await?;
        }
        Commands::TestNotifications { channel } => {
            test_notifications_command(config_path, channel).await?;
        }
        Commands::ValidateConfig => {
            validate_config_command(config_path).await?;
        }
        Commands::Rules { action } => match action {
            RuleAction::List => {
                rules_list_command().await?;
            }
            RuleAction::Info { rule_name } => {
                rules_info_command(rule_name).await?;
            }
            RuleAction::Test { rule_name } => {
                rules_test_command(rule_name).await?;
            }
        },
        Commands::Status => {
            status_command().await?;
        }
        Commands::Stop => {
            stop_command().await?;
        }
    }

    Ok(())
}

fn init_logging(verbose: bool, debug: bool) -> Result<()> {
    let level = if debug {
        Level::DEBUG
    } else if verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    Ok(())
}

fn print_banner() {
    println!("{}", style("").bold());
    println!(
        "{}",
        style("🛡️  Solana Watchtower v").bold().cyan().to_string()
            + &style(env!("CARGO_PKG_VERSION")).bold().white().to_string()
    );
    println!(
        "{}",
        style("   Real-time monitoring for Solana programs").dim()
    );
    println!();
}
