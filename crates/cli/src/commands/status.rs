use anyhow::Result;
use console::style;

pub async fn status_command() -> Result<()> {
    println!("{}", style("Watchtower System Status").bold().cyan());
    println!("{}", "─".repeat(50));

    // Check if watchtower process is running
    let is_running = check_process_running().await;

    if is_running {
        println!(
            "{} {}",
            style("Status:").bold(),
            style("Running").green().bold()
        );
    } else {
        println!(
            "{} {}",
            style("Status:").bold(),
            style("Not Running").red().bold()
        );
    }

    // Try to get metrics from running instance
    if is_running {
        match get_metrics().await {
            Ok(metrics) => {
                println!("\n{}", style("Metrics:").bold());
                println!(
                    "• Events processed: {}",
                    style(&metrics.events_processed).cyan()
                );
                println!(
                    "• Alerts generated: {}",
                    style(&metrics.alerts_generated).cyan()
                );
                println!("• Rules active: {}", style(&metrics.active_rules).cyan());
                println!("• Uptime: {}", style(&metrics.uptime).cyan());

                if !metrics.connected_endpoints.is_empty() {
                    println!("\n{}", style("Connected Endpoints:").bold());
                    for endpoint in &metrics.connected_endpoints {
                        println!("• {}", style(endpoint).cyan());
                    }
                }

                if !metrics.notification_channels.is_empty() {
                    println!("\n{}", style("Notification Channels:").bold());
                    for (channel, status) in &metrics.notification_channels {
                        let status_style = if status == "active" {
                            style(status).green()
                        } else {
                            style(status).red()
                        };
                        println!("• {}: {}", style(channel).cyan(), status_style);
                    }
                }
            }
            Err(e) => {
                println!("\n{} Failed to get metrics: {}", style("⚠️").yellow(), e);
            }
        }

        // Show dashboard and metrics URLs
        println!("\n{}", style("Endpoints:").bold());
        println!("• Dashboard: {}", style("http://127.0.0.1:8080").cyan());
        println!(
            "• Metrics: {}",
            style("http://127.0.0.1:9090/metrics").cyan()
        );
    } else {
        println!(
            "\n{}",
            style("The watchtower service is not currently running.").dim()
        );
        println!(
            "{}",
            style("Use 'watchtower start' to begin monitoring.").dim()
        );
    }

    // Check configuration
    match check_configuration().await {
        Ok(config_status) => {
            println!("\n{}", style("Configuration:").bold());
            println!(
                "• Config file: {}",
                if config_status.exists {
                    style("Found").green()
                } else {
                    style("Not found").red()
                }
            );
            if config_status.exists {
                println!(
                    "• Programs monitored: {}",
                    style(&config_status.programs_count).cyan()
                );
                println!(
                    "• Notification channels: {}",
                    style(&config_status.channels_count).cyan()
                );
            }
        }
        Err(e) => {
            println!(
                "\n{} Configuration check failed: {}",
                style("⚠️").yellow(),
                e
            );
        }
    }

    // Show recent logs if available
    if is_running {
        if let Ok(logs) = get_recent_logs().await {
            if !logs.is_empty() {
                println!("\n{}", style("Recent Activity:").bold());
                for log in logs.iter().take(5) {
                    let level_style = match log.level.as_str() {
                        "ERROR" => style(&log.level).red(),
                        "WARN" => style(&log.level).yellow(),
                        "INFO" => style(&log.level).green(),
                        _ => style(&log.level).dim(),
                    };
                    println!(
                        "• {} [{}] {}",
                        style(&log.timestamp).dim(),
                        level_style,
                        &log.message
                    );
                }
            }
        }
    }

    Ok(())
}

async fn check_process_running() -> bool {
    // Try to connect to the metrics endpoint
    match reqwest::get("http://127.0.0.1:9090/metrics").await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

#[derive(Debug)]
struct SystemMetrics {
    events_processed: String,
    alerts_generated: String,
    active_rules: String,
    uptime: String,
    connected_endpoints: Vec<String>,
    notification_channels: Vec<(String, String)>,
}

async fn get_metrics() -> Result<SystemMetrics> {
    // In a real implementation, this would parse Prometheus metrics
    // For now, return mock data
    Ok(SystemMetrics {
        events_processed: "1,234".to_string(),
        alerts_generated: "12".to_string(),
        active_rules: "4".to_string(),
        uptime: "2h 15m".to_string(),
        connected_endpoints: vec!["wss://api.mainnet-beta.solana.com".to_string()],
        notification_channels: vec![
            ("email".to_string(), "active".to_string()),
            ("telegram".to_string(), "active".to_string()),
        ],
    })
}

#[derive(Debug)]
struct ConfigStatus {
    exists: bool,
    programs_count: String,
    channels_count: String,
}

async fn check_configuration() -> Result<ConfigStatus> {
    let config_path = dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join("watchtower.toml");

    let exists = config_path.exists();

    if exists {
        // Try to load and count configurations
        match crate::config::AppConfig::load_from_file(&config_path) {
            Ok(config) => {
                let channels = config.notifier.enabled_channels();
                Ok(ConfigStatus {
                    exists: true,
                    programs_count: config.subscriber.programs.len().to_string(),
                    channels_count: channels.len().to_string(),
                })
            }
            Err(_) => Ok(ConfigStatus {
                exists: true,
                programs_count: "Invalid config".to_string(),
                channels_count: "Invalid config".to_string(),
            }),
        }
    } else {
        Ok(ConfigStatus {
            exists: false,
            programs_count: "0".to_string(),
            channels_count: "0".to_string(),
        })
    }
}

#[derive(Debug)]
struct LogEntry {
    timestamp: String,
    level: String,
    message: String,
}

async fn get_recent_logs() -> Result<Vec<LogEntry>> {
    // In a real implementation, this would read from log files or query a logging system
    // For now, return mock recent activity
    Ok(vec![
        LogEntry {
            timestamp: "2024-01-15 10:30:15".to_string(),
            level: "INFO".to_string(),
            message: "Monitoring engine started successfully".to_string(),
        },
        LogEntry {
            timestamp: "2024-01-15 10:30:20".to_string(),
            level: "INFO".to_string(),
            message: "Connected to Solana WebSocket".to_string(),
        },
        LogEntry {
            timestamp: "2024-01-15 10:32:45".to_string(),
            level: "WARN".to_string(),
            message: "Large transaction detected: 50 SOL transfer".to_string(),
        },
        LogEntry {
            timestamp: "2024-01-15 10:35:12".to_string(),
            level: "INFO".to_string(),
            message: "Alert notification sent via email".to_string(),
        },
    ])
}
