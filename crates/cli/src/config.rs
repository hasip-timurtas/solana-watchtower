use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use watchtower_engine::EngineConfig;
use watchtower_notifier::NotifierConfig;
use watchtower_subscriber::SubscriberConfig;

/// Main application configuration that combines all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Subscriber configuration for monitoring
    #[serde(flatten)]
    pub subscriber: SubscriberConfig,

    /// Engine configuration for rule processing
    #[serde(default)]
    pub engine: EngineConfig,

    /// Notification configuration
    #[serde(flatten)]
    pub notifier: NotifierConfig,

    /// Dashboard configuration
    #[serde(default)]
    pub dashboard: DashboardConfig,

    /// General application settings
    #[serde(default)]
    pub app: AppSettings,
}

/// Dashboard-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Whether to enable the web dashboard
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Port for the dashboard server
    #[serde(default = "default_dashboard_port")]
    pub port: u16,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Whether to enable CORS
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// Static files directory (optional)
    pub static_dir: Option<String>,
}

/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// PID file location for daemon mode
    #[serde(default)]
    pub pid_file: Option<String>,

    /// Working directory
    #[serde(default)]
    pub working_dir: Option<String>,

    /// Maximum number of worker threads
    #[serde(default)]
    pub max_threads: Option<usize>,
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path).with_context(|| {
            format!("Failed to read config file: {}", path.as_ref().display())
        })?;

        let mut config: AppConfig = toml::from_str(&content).with_context(|| {
            format!("Failed to parse config file: {}", path.as_ref().display())
        })?;

        // Validate the configuration
        config.validate().with_context(|| {
            format!("Invalid configuration in: {}", path.as_ref().display())
        })?;

        Ok(config)
    }

    /// Load configuration from environment and file
    pub fn load_with_overrides<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut config = Self::load_from_file(path)?;

        // Override with environment variables
        config.apply_env_overrides();

        config.validate()?;
        Ok(config)
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> Result<()> {
        // Validate subscriber config
        self.subscriber
            .validate()
            .context("Invalid subscriber configuration")?;

        // Validate notifier config
        self.notifier
            .validate()
            .context("Invalid notifier configuration")?;

        // Validate dashboard config
        self.dashboard
            .validate()
            .context("Invalid dashboard configuration")?;

        Ok(())
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Override log level
        if let Ok(log_level) = std::env::var("WATCHTOWER_LOG_LEVEL") {
            self.app.log_level = log_level;
        }

        // Override dashboard port
        if let Ok(port_str) = std::env::var("WATCHTOWER_DASHBOARD_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                self.dashboard.port = port;
            }
        }

        // Override RPC URL
        if let Ok(rpc_url) = std::env::var("WATCHTOWER_RPC_URL") {
            if let Ok(url) = rpc_url.parse() {
                self.subscriber.rpc_url = url;
            }
        }

        // Override WebSocket URL
        if let Ok(ws_url) = std::env::var("WATCHTOWER_WS_URL") {
            if let Ok(url) = ws_url.parse() {
                self.subscriber.ws_url = url;
            }
        }

        // Override email password (sensitive)
        if let Ok(password) = std::env::var("WATCHTOWER_EMAIL_PASSWORD") {
            if let Some(email_config) = &mut self.notifier.email {
                email_config.password = password;
            }
        }

        // Override Telegram bot token (sensitive)
        if let Ok(token) = std::env::var("WATCHTOWER_TELEGRAM_TOKEN") {
            if let Some(telegram_config) = &mut self.notifier.telegram {
                telegram_config.bot_token = token;
            }
        }
    }

    /// Create a default configuration for testing
    pub fn default_for_testing() -> Self {
        Self {
            subscriber: SubscriberConfig {
                rpc_url: "https://api.devnet.solana.com".parse().unwrap(),
                ws_url: "wss://api.devnet.solana.com".parse().unwrap(),
                timeout_seconds: 30,
                max_reconnect_attempts: 3,
                reconnect_delay_seconds: 5,
                programs: vec![],
                filters: Default::default(),
            },
            engine: EngineConfig::default(),
            notifier: NotifierConfig {
                email: None,
                telegram: None,
                slack: None,
                discord: None,
                rate_limiting: Default::default(),
                global: Default::default(),
            },
            dashboard: DashboardConfig::default(),
            app: AppSettings::default(),
        }
    }
}

impl DashboardConfig {
    fn validate(&self) -> Result<()> {
        if self.port == 0 {
            anyhow::bail!("Dashboard port cannot be 0");
        }

        if self.host.is_empty() {
            anyhow::bail!("Dashboard host cannot be empty");
        }

        Ok(())
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            port: default_dashboard_port(),
            host: default_host(),
            enable_cors: default_true(),
            static_dir: None,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            pid_file: None,
            working_dir: None,
            max_threads: None,
        }
    }
}

// Default value functions
fn default_true() -> bool {
    true
}

fn default_dashboard_port() -> u16 {
    8080
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_loading() {
        let config_content = r#"
            rpc_url = "https://api.mainnet-beta.solana.com"
            ws_url = "wss://api.mainnet-beta.solana.com"

            [[programs]]
            id = "TokenkegQfeGuoRqH9L4g1hxgCaLJaFgqhk5eHwUfVR"
            name = "SPL Token"

            [dashboard]
            enabled = true
            port = 3000

            [app]
            log_level = "debug"
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", config_content).unwrap();

        let config = AppConfig::load_from_file(temp_file.path()).unwrap();
        assert_eq!(config.dashboard.port, 3000);
        assert_eq!(config.app.log_level, "debug");
        assert_eq!(config.subscriber.programs.len(), 1);
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("WATCHTOWER_LOG_LEVEL", "trace");
        std::env::set_var("WATCHTOWER_DASHBOARD_PORT", "9999");

        let mut config = AppConfig::default_for_testing();
        config.apply_env_overrides();

        assert_eq!(config.app.log_level, "trace");
        assert_eq!(config.dashboard.port, 9999);

        // Cleanup
        std::env::remove_var("WATCHTOWER_LOG_LEVEL");
        std::env::remove_var("WATCHTOWER_DASHBOARD_PORT");
    }
} 