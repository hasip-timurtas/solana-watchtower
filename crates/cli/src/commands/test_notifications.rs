use crate::config::AppConfig;
use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;
use watchtower_notifier::NotificationManager;

pub async fn test_notifications_command(
    config_path: PathBuf,
    channel: Option<String>,
) -> Result<()> {
    println!("{}", style("Loading configuration...").cyan());

    // Load configuration
    let config = AppConfig::load_with_overrides(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path.display()))?;

    println!("{}", style("✓ Configuration loaded").green());

    // Create notification manager
    let notification_manager = NotificationManager::new(config.notifier.clone())
        .await
        .context("Failed to create notification manager")?;

    println!("{}", style("Testing notification channels...").cyan());

    // Test all configured channels first
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} Testing channels...")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    let all_results = notification_manager.test_channels().await;

    pb.finish_and_clear();

    // Filter results if specific channel requested
    let results = if let Some(channel_name) = &channel {
        let mut filtered_results = std::collections::HashMap::new();

        if let Some(result) = all_results.get(channel_name) {
            filtered_results.insert(channel_name.clone(), result);
        } else {
            anyhow::bail!("Channel '{}' is not configured", channel_name);
        }

        filtered_results
    } else {
        all_results
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect::<std::collections::HashMap<_, _>>()
    };

    // Display results
    println!("\n{}", style("Test Results:").bold());
    println!("{}", "─".repeat(50));

    let mut success_count = 0;
    let mut total_count = 0;

    for (channel_name, result) in &results {
        total_count += 1;

        match result {
            Ok(_) => {
                success_count += 1;
                println!(
                    "{} {} {}",
                    style("✓").green().bold(),
                    style(format!("{:12}", channel_name)).cyan(),
                    style("Test passed").green()
                );
            }
            Err(e) => {
                println!(
                    "{} {} {} {}",
                    style("✗").red().bold(),
                    style(format!("{:12}", channel_name)).cyan(),
                    style("Test failed:").red(),
                    style(format!("{}", e)).red().dim()
                );
            }
        }
    }

    println!("{}", "─".repeat(50));

    if success_count == total_count {
        println!(
            "{} All {} channel(s) tested successfully!",
            style("🎉").bold(),
            total_count
        );
    } else {
        println!(
            "{} {}/{} channel(s) passed tests",
            if success_count > 0 {
                style("⚠️").yellow()
            } else {
                style("❌").red()
            },
            success_count,
            total_count
        );

        if success_count == 0 {
            println!("\n{}", style("Troubleshooting tips:").bold().yellow());
            println!("• Check your configuration file for correct credentials");
            println!("• Verify network connectivity");
            println!("• Ensure API tokens/passwords are valid and not expired");
            println!("• Check firewall settings for outbound connections");
        }
    }

    // Show notification statistics if available
    let stats = notification_manager.statistics().await;
    if stats.total_sent > 0 || stats.total_failed > 0 {
        println!("\n{}", style("Notification Statistics:").bold());
        println!("Total sent: {}", stats.total_sent);
        println!("Total failed: {}", stats.total_failed);
        println!("Rate limited: {}", stats.rate_limited);

        if !stats.sent_per_channel.is_empty() {
            println!("\nPer channel:");
            for (channel, count) in &stats.sent_per_channel {
                println!("  {}: {}", channel, count);
            }
        }
    }

    if success_count < total_count {
        std::process::exit(1);
    }

    Ok(())
}
