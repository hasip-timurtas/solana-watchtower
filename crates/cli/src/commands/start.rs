use crate::config::AppConfig;
use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use watchtower_engine::{MonitoringEngine, AlertManager, MetricsCollector};
use watchtower_notifier::NotificationManager;
use watchtower_subscriber::SolanaWebSocketClient;

pub async fn start_command(
    config_path: PathBuf,
    daemon: bool,
    dashboard_port: u16,
    metrics_port: u16,
) -> Result<()> {
    println!("{}", style("Loading configuration...").cyan());

    // Load configuration
    let mut config = AppConfig::load_with_overrides(&config_path)
        .with_context(|| format!("Failed to load config from {}", config_path.display()))?;

    // Override ports from command line
    config.dashboard.port = dashboard_port;

    println!("{}", style("✓ Configuration loaded successfully").green());

    if daemon {
        println!("{}", style("Starting in daemon mode...").cyan());
        daemonize(&config)?;
    }

    // Initialize components
    println!("{}", style("Initializing monitoring components...").cyan());

    // Create metrics collector
    let metrics = Arc::new(
        MetricsCollector::new()
            .context("Failed to create metrics collector")?
    );

    // Create alert manager
    let alert_manager = Arc::new(AlertManager::new());

    // Create monitoring engine
    let engine = Arc::new(
        MonitoringEngine::new(
            metrics.clone(),
            alert_manager.clone(),
            config.engine.clone(),
        )
    );

    // Create notification manager
    let notification_manager = Arc::new(
        NotificationManager::new(config.notifier.clone())
            .await
            .context("Failed to create notification manager")?
    );

    // Create WebSocket subscriber
    let mut subscriber = SolanaWebSocketClient::new(config.subscriber.clone())
        .context("Failed to create WebSocket client")?;

    println!("{}", style("✓ Components initialized").green());

    // Register built-in rules
    register_builtin_rules(&engine).await?;

    // Start the monitoring engine
    engine.start().await.context("Failed to start monitoring engine")?;
    println!("{}", style("✓ Monitoring engine started").green());

    // Start the subscriber and get event receiver
    let mut event_receiver = subscriber.start().await
        .context("Failed to start WebSocket subscriber")?;
    println!("{}", style("✓ WebSocket subscriber started").green());

    // Subscribe to alerts and connect to notification manager
    let mut alert_receiver = engine.subscribe_to_alerts();
    let notification_manager_clone = notification_manager.clone();
    tokio::spawn(async move {
        while let Ok(alert) = alert_receiver.recv().await {
            if let Err(e) = notification_manager_clone.send_notification(alert).await {
                error!("Failed to send notification: {}", e);
            }
        }
    });

    // Start dashboard if enabled
    if config.dashboard.enabled {
        let dashboard_config = config.dashboard.clone();
        let engine_clone = engine.clone();
        let alert_manager_clone = alert_manager.clone();
        
        tokio::spawn(async move {
            if let Err(e) = start_dashboard(dashboard_config, engine_clone, alert_manager_clone).await {
                error!("Dashboard error: {}", e);
            }
        });
        
        println!(
            "{} {}",
            style("✓ Dashboard started on").green(),
            style(format!("http://{}:{}", config.dashboard.host, config.dashboard.port)).bold()
        );
    }

    // Start metrics server
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        if let Err(e) = start_metrics_server(metrics_clone, metrics_port).await {
            error!("Metrics server error: {}", e);
        }
    });

    println!(
        "{} {}",
        style("✓ Metrics server started on").green(),
        style(format!("http://127.0.0.1:{}/metrics", metrics_port)).bold()
    );

    // Main event processing loop
    println!("{}", style("🛡️  Watchtower is now monitoring Solana programs").bold().green());
    println!("{}", style("Press Ctrl+C to stop").dim());

    // Event processing task
    let engine_clone = engine.clone();
    let event_task = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            if let Err(e) = engine_clone.process_event(event).await {
                error!("Error processing event: {}", e);
            }
        }
    });

    // Wait for shutdown signal
    let shutdown_signal = signal::ctrl_c();
    tokio::select! {
        _ = shutdown_signal => {
            info!("Shutdown signal received");
        }
        _ = event_task => {
            warn!("Event processing task ended unexpectedly");
        }
    }

    // Graceful shutdown
    println!("{}", style("Shutting down...").yellow());

    // Stop components
    engine.stop().await.context("Failed to stop monitoring engine")?;
    notification_manager.shutdown().await.context("Failed to shutdown notification manager")?;

    println!("{}", style("✓ Watchtower stopped").green());
    Ok(())
}

async fn register_builtin_rules(engine: &MonitoringEngine) -> Result<()> {
    use watchtower_engine::{
        LiquidityDropRule, LargeTransactionRule, OracleDeviationRule, FailureRateRule
    };

    // Register built-in rules
    engine.add_rule(Box::new(LiquidityDropRule::new(10.0, 300, 1000000))).await;
    engine.add_rule(Box::new(LargeTransactionRule::new(1.0, 500000))).await;
    engine.add_rule(Box::new(OracleDeviationRule::new(5.0, "reference_oracle".to_string()))).await;
    engine.add_rule(Box::new(FailureRateRule::new(25.0, 10, 300))).await;

    info!("Registered {} built-in rules", engine.list_rules().await.len());
    Ok(())
}

async fn start_dashboard(
    _config: crate::config::DashboardConfig,
    _engine: Arc<MonitoringEngine>,
    _alert_manager: Arc<AlertManager>,
) -> Result<()> {
    // Dashboard implementation would go here
    // For now, we'll just log that it's started
    info!("Dashboard server started (implementation pending)");
    
    // Keep the task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

async fn start_metrics_server(
    metrics: Arc<MetricsCollector>,
    port: u16,
) -> Result<()> {
    use std::convert::Infallible;
    use std::net::SocketAddr;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    let make_svc = hyper::service::make_service_fn(move |_conn| {
        let metrics = metrics.clone();
        async move {
            Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                let metrics = metrics.clone();
                async move {
                    if req.uri().path() == "/metrics" {
                        let body = metrics.export();
                        Ok::<_, Infallible>(
                            hyper::Response::builder()
                                .header("content-type", "text/plain; version=0.0.4")
                                .body(hyper::Body::from(body))
                                .unwrap()
                        )
                    } else {
                        Ok(hyper::Response::builder()
                            .status(404)
                            .body(hyper::Body::from("Not Found"))
                            .unwrap())
                    }
                }
            }))
        }
    });

    let server = hyper::Server::bind(&addr).serve(make_svc);
    
    info!("Metrics server listening on {}", addr);
    
    if let Err(e) = server.await {
        error!("Metrics server error: {}", e);
    }

    Ok(())
}

fn daemonize(config: &AppConfig) -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs::File;
        use std::os::unix::prelude::*;

        // Fork the process
        let pid = unsafe { libc::fork() };
        
        if pid < 0 {
            anyhow::bail!("Failed to fork process");
        } else if pid > 0 {
            // Parent process exits
            std::process::exit(0);
        }

        // Child process continues
        // Create new session
        if unsafe { libc::setsid() } < 0 {
            anyhow::bail!("Failed to create new session");
        }

        // Change working directory
        if let Some(work_dir) = &config.app.working_dir {
            std::env::set_current_dir(work_dir)
                .context("Failed to change working directory")?;
        }

        // Redirect standard streams
        let null_fd = File::open("/dev/null")?.as_raw_fd();
        unsafe {
            libc::dup2(null_fd, 0); // stdin
            libc::dup2(null_fd, 1); // stdout
            libc::dup2(null_fd, 2); // stderr
        }

        // Write PID file if specified
        if let Some(pid_file) = &config.app.pid_file {
            std::fs::write(pid_file, format!("{}", std::process::id()))
                .context("Failed to write PID file")?;
        }

        info!("Daemonized with PID: {}", std::process::id());
    }

    #[cfg(not(unix))]
    {
        warn!("Daemon mode not supported on this platform");
    }

    Ok(())
} 