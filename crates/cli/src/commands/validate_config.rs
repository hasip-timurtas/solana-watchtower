use crate::config::AppConfig;
use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;

pub async fn validate_config_command(config_path: PathBuf) -> Result<()> {
    println!(
        "{} {}",
        style("Validating configuration:").cyan(),
        style(config_path.display()).bold()
    );
    println!();

    // Check if file exists
    if !config_path.exists() {
        println!(
            "{} Configuration file not found: {}",
            style("✗").red().bold(),
            config_path.display()
        );
        std::process::exit(1);
    }

    println!("{} File exists", style("✓").green());

    // Try to load and parse the configuration
    let config = match AppConfig::load_with_overrides(&config_path) {
        Ok(config) => {
            println!("{} TOML syntax is valid", style("✓").green());
            config
        }
        Err(e) => {
            println!(
                "{} TOML parsing failed: {}",
                style("✗").red().bold(),
                style(format!("{}", e)).red()
            );
            std::process::exit(1);
        }
    };

    // Validate individual components
    validate_subscriber_config(&config).await?;
    validate_engine_config(&config).await?;
    validate_notifier_config(&config).await?;
    validate_dashboard_config(&config).await?;

    // Summary
    println!();
    println!("{}", style("Configuration Summary:").bold());
    println!("{}", "─".repeat(40));

    // Subscriber info
    println!(
        "RPC URL: {}",
        style(&config.subscriber.rpc_url).cyan()
    );
    println!(
        "WebSocket URL: {}",
        style(&config.subscriber.ws_url).cyan()
    );
    println!(
        "Programs monitored: {}",
        style(config.subscriber.programs.len()).cyan()
    );

    // Notification channels
    let enabled_channels = config.notifier.enabled_channels();
    if enabled_channels.is_empty() {
        println!(
            "Notification channels: {}",
            style("None configured").yellow()
        );
    } else {
        println!(
            "Notification channels: {}",
            style(enabled_channels.join(", ")).cyan()
        );
    }

    // Dashboard
    if config.dashboard.enabled {
        println!(
            "Dashboard: {} ({}:{})",
            style("Enabled").green(),
            config.dashboard.host,
            config.dashboard.port
        );
    } else {
        println!("Dashboard: {}", style("Disabled").dim());
    }

    println!("{}", "─".repeat(40));
    println!(
        "{} Configuration is valid and ready to use!",
        style("🎉").bold()
    );

    Ok(())
}

async fn validate_subscriber_config(config: &AppConfig) -> Result<()> {
    println!("{}", style("Validating subscriber configuration...").cyan());

    // Validate URLs
    let rpc_url = &config.subscriber.rpc_url;
    let ws_url = &config.subscriber.ws_url;

    if rpc_url.scheme() != "https" && rpc_url.scheme() != "http" {
        anyhow::bail!("RPC URL must use http or https scheme");
    }

    if ws_url.scheme() != "wss" && ws_url.scheme() != "ws" {
        anyhow::bail!("WebSocket URL must use ws or wss scheme");
    }

    println!("{} URLs are valid", style("✓").green());

    // Validate programs
    if config.subscriber.programs.is_empty() {
        println!(
            "{} No programs configured for monitoring",
            style("⚠️").yellow()
        );
    } else {
        for program in &config.subscriber.programs {
            if program.name.is_empty() {
                anyhow::bail!("Program {} has empty name", program.id);
            }

            if !program.has_monitoring_enabled() {
                println!(
                    "{} Program '{}' has no monitoring enabled",
                    style("⚠️").yellow(),
                    program.name
                );
            }
        }
        println!(
            "{} {} program(s) configured",
            style("✓").green(),
            config.subscriber.programs.len()
        );
    }

    // Validate timeouts
    if config.subscriber.timeout_seconds == 0 {
        anyhow::bail!("Timeout cannot be zero");
    }

    if config.subscriber.max_reconnect_attempts == 0 {
        println!(
            "{} Reconnection is disabled (max_reconnect_attempts = 0)",
            style("⚠️").yellow()
        );
    }

    println!("{} Subscriber configuration is valid", style("✓").green());
    Ok(())
}

async fn validate_engine_config(config: &AppConfig) -> Result<()> {
    println!("{}", style("Validating engine configuration...").cyan());

    // Validate history settings
    if config.engine.max_history_events == 0 {
        println!(
            "{} Event history is disabled (max_history_events = 0)",
            style("⚠️").yellow()
        );
    }

    if config.engine.max_concurrent_evaluations == 0 {
        anyhow::bail!("max_concurrent_evaluations cannot be zero");
    }

    if config.engine.rule_timeout.as_secs() == 0 {
        anyhow::bail!("rule_timeout cannot be zero");
    }

    println!("{} Engine configuration is valid", style("✓").green());
    Ok(())
}

async fn validate_notifier_config(config: &AppConfig) -> Result<()> {
    println!("{}", style("Validating notifier configuration...").cyan());

    let enabled_channels = config.notifier.enabled_channels();
    
    if enabled_channels.is_empty() {
        println!(
            "{} No notification channels configured",
            style("⚠️").yellow()
        );
        return Ok(());
    }

    // Validate rate limiting
    if config.notifier.rate_limiting.enabled {
        if config.notifier.rate_limiting.max_messages_per_minute == 0 {
            anyhow::bail!("max_messages_per_minute cannot be zero when rate limiting is enabled");
        }
    }

    // Check notification filters
    if let Some(filters) = &config.notifier.global.filters {
        for filter in filters {
            if filter.name.is_empty() {
                anyhow::bail!("Filter cannot have empty name");
            }
        }
    }

    println!(
        "{} Notifier configuration is valid ({} channels)",
        style("✓").green(),
        enabled_channels.len()
    );
    Ok(())
}

async fn validate_dashboard_config(config: &AppConfig) -> Result<()> {
    println!("{}", style("Validating dashboard configuration...").cyan());

    if !config.dashboard.enabled {
        println!("{} Dashboard is disabled", style("ⓘ").blue());
        return Ok(());
    }

    if config.dashboard.port == 0 {
        anyhow::bail!("Dashboard port cannot be zero");
    }

    if config.dashboard.host.is_empty() {
        anyhow::bail!("Dashboard host cannot be empty");
    }

    // Check for port conflicts
    if config.dashboard.port == 9090 {
        println!(
            "{} Dashboard port conflicts with default metrics port (9090)",
            style("⚠️").yellow()
        );
    }

    println!("{} Dashboard configuration is valid", style("✓").green());
    Ok(())
} 