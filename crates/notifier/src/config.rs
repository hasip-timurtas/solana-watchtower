//! Configuration structures for notification channels.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration for the notification system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifierConfig {
    /// Email notification configuration
    pub email: Option<EmailConfig>,

    /// Telegram notification configuration
    pub telegram: Option<TelegramConfig>,

    /// Slack notification configuration
    pub slack: Option<SlackConfig>,

    /// Discord notification configuration
    pub discord: Option<DiscordConfig>,

    /// Rate limiting configuration
    #[serde(default)]
    pub rate_limiting: RateLimitConfig,

    /// Global notification settings
    #[serde(default)]
    pub global: GlobalNotificationConfig,
}

/// Email notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP server hostname
    pub smtp_server: String,

    /// SMTP server port
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// Username for SMTP authentication
    pub username: String,

    /// Password for SMTP authentication
    pub password: String,

    /// From email address
    pub from_address: String,

    /// From name (optional)
    pub from_name: Option<String>,

    /// List of recipient email addresses
    pub to_addresses: Vec<String>,

    /// Use TLS encryption
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// Email subject template
    pub subject_template: Option<String>,

    /// Email body template (HTML or plain text)
    pub body_template: Option<String>,
}

/// Telegram notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Telegram Bot API token
    pub bot_token: String,

    /// Chat ID to send messages to
    pub chat_id: i64,

    /// Message template
    pub message_template: Option<String>,

    /// Parse mode (Markdown, HTML, or None)
    #[serde(default = "default_parse_mode")]
    pub parse_mode: String,

    /// Disable web page preview
    #[serde(default)]
    pub disable_web_page_preview: bool,

    /// Send messages silently
    #[serde(default)]
    pub disable_notification: bool,
}

/// Slack notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Slack webhook URL
    pub webhook_url: String,

    /// Channel to send messages to (optional, webhook may have default)
    pub channel: Option<String>,

    /// Username to send messages as
    pub username: Option<String>,

    /// Icon emoji or URL for the bot
    pub icon: Option<String>,

    /// Message template
    pub message_template: Option<String>,

    /// Custom fields to include in messages
    pub custom_fields: Option<HashMap<String, String>>,
}

/// Discord notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Discord webhook URL
    pub webhook_url: String,

    /// Username to send messages as
    pub username: Option<String>,

    /// Avatar URL for the bot
    pub avatar_url: Option<String>,

    /// Message template
    pub message_template: Option<String>,

    /// Whether to use Discord embeds for rich formatting
    #[serde(default = "default_true")]
    pub use_embeds: bool,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum messages per minute per channel
    #[serde(default = "default_max_messages_per_minute")]
    pub max_messages_per_minute: u32,

    /// Maximum burst size (messages that can be sent immediately)
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Whether to enable rate limiting
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Global notification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalNotificationConfig {
    /// Minimum severity level to send notifications
    #[serde(default = "default_min_severity")]
    pub min_severity: String,

    /// Maximum number of notifications to batch together
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Batch timeout in seconds
    #[serde(default = "default_batch_timeout")]
    pub batch_timeout_seconds: u64,

    /// Whether to enable notification batching
    #[serde(default)]
    pub enable_batching: bool,

    /// Custom notification filters
    pub filters: Option<Vec<NotificationFilter>>,
}

/// Notification filter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationFilter {
    /// Filter name
    pub name: String,

    /// Rule names to include/exclude
    pub rule_names: Option<Vec<String>>,

    /// Program names to include/exclude
    pub program_names: Option<Vec<String>>,

    /// Severity levels to include/exclude
    pub severities: Option<Vec<String>>,

    /// Whether this is an include filter (true) or exclude filter (false)
    #[serde(default = "default_true")]
    pub include: bool,

    /// Channels to apply this filter to
    pub channels: Option<Vec<String>>,
}

impl NotifierConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> crate::NotifierResult<()> {
        // Validate email config
        if let Some(email) = &self.email {
            email.validate()?;
        }

        // Validate Telegram config
        if let Some(telegram) = &self.telegram {
            telegram.validate()?;
        }

        // Validate Slack config
        if let Some(slack) = &self.slack {
            slack.validate()?;
        }

        // Validate Discord config
        if let Some(discord) = &self.discord {
            discord.validate()?;
        }

        // Check that at least one notification channel is configured
        if self.email.is_none()
            && self.telegram.is_none()
            && self.slack.is_none()
            && self.discord.is_none()
        {
            return Err(crate::NotifierError::Configuration(
                "At least one notification channel must be configured".to_string(),
            ));
        }

        Ok(())
    }

    /// Get all configured channel names.
    pub fn enabled_channels(&self) -> Vec<String> {
        let mut channels = Vec::new();

        if self.email.is_some() {
            channels.push("email".to_string());
        }
        if self.telegram.is_some() {
            channels.push("telegram".to_string());
        }
        if self.slack.is_some() {
            channels.push("slack".to_string());
        }
        if self.discord.is_some() {
            channels.push("discord".to_string());
        }

        channels
    }
}

impl EmailConfig {
    fn validate(&self) -> crate::NotifierResult<()> {
        if self.smtp_server.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "SMTP server cannot be empty".to_string(),
            ));
        }

        if self.username.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "SMTP username cannot be empty".to_string(),
            ));
        }

        if self.password.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "SMTP password cannot be empty".to_string(),
            ));
        }

        if self.from_address.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "From address cannot be empty".to_string(),
            ));
        }

        if self.to_addresses.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "At least one recipient address must be specified".to_string(),
            ));
        }

        Ok(())
    }
}

impl TelegramConfig {
    fn validate(&self) -> crate::NotifierResult<()> {
        if self.bot_token.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "Telegram bot token cannot be empty".to_string(),
            ));
        }

        if !["Markdown", "HTML", ""].contains(&self.parse_mode.as_str()) {
            return Err(crate::NotifierError::Configuration(
                "Invalid Telegram parse mode. Must be 'Markdown', 'HTML', or empty".to_string(),
            ));
        }

        Ok(())
    }
}

impl SlackConfig {
    fn validate(&self) -> crate::NotifierResult<()> {
        if self.webhook_url.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "Slack webhook URL cannot be empty".to_string(),
            ));
        }

        if !self.webhook_url.starts_with("https://hooks.slack.com/") {
            return Err(crate::NotifierError::Configuration(
                "Invalid Slack webhook URL format".to_string(),
            ));
        }

        Ok(())
    }
}

impl DiscordConfig {
    fn validate(&self) -> crate::NotifierResult<()> {
        if self.webhook_url.is_empty() {
            return Err(crate::NotifierError::Configuration(
                "Discord webhook URL cannot be empty".to_string(),
            ));
        }

        if !self
            .webhook_url
            .starts_with("https://discord.com/api/webhooks/")
        {
            return Err(crate::NotifierError::Configuration(
                "Invalid Discord webhook URL format".to_string(),
            ));
        }

        Ok(())
    }
}

// Default value functions
fn default_smtp_port() -> u16 {
    587
}

fn default_true() -> bool {
    true
}

fn default_parse_mode() -> String {
    "Markdown".to_string()
}

fn default_max_messages_per_minute() -> u32 {
    10
}

fn default_burst_size() -> u32 {
    5
}

fn default_min_severity() -> String {
    "medium".to_string()
}

fn default_batch_size() -> usize {
    5
}

fn default_batch_timeout() -> u64 {
    60
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_minute: default_max_messages_per_minute(),
            burst_size: default_burst_size(),
            enabled: default_true(),
        }
    }
}

impl Default for GlobalNotificationConfig {
    fn default() -> Self {
        Self {
            min_severity: default_min_severity(),
            batch_size: default_batch_size(),
            batch_timeout_seconds: default_batch_timeout(),
            enable_batching: false,
            filters: None,
        }
    }
}
