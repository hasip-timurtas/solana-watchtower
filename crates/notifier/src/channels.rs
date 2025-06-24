//! Notification channel implementations.

use crate::{
    config::{DiscordConfig, EmailConfig, SlackConfig, TelegramConfig},
    error::{NotifierError, NotifierResult},
    templates::TemplateEngine,
};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, Message},
    transport::smtp::{authentication::Credentials, PoolConfig},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{debug, error, info};
use watchtower_engine::Alert;

/// Trait for notification channels.
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Channel name (e.g., "email", "telegram", "slack")
    fn name(&self) -> &str;

    /// Send a notification through this channel
    async fn send(&self, alert: &Alert, template_data: &HashMap<String, Value>) -> NotifierResult<()>;

    /// Test the channel configuration
    async fn test(&self) -> NotifierResult<()>;

    /// Whether this channel supports batching
    fn supports_batching(&self) -> bool {
        false
    }

    /// Send multiple alerts as a batch (if supported)
    async fn send_batch(&self, _alerts: &[Alert], _template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        Err(NotifierError::Generic("Batching not supported for this channel".to_string()))
    }
}

/// Email notification channel.
pub struct EmailChannel {
    config: EmailConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    template_engine: TemplateEngine,
}

/// Telegram notification channel.
pub struct TelegramChannel {
    config: TelegramConfig,
    client: Client,
    template_engine: TemplateEngine,
}

/// Slack notification channel.
pub struct SlackChannel {
    config: SlackConfig,
    client: Client,
    template_engine: TemplateEngine,
}

/// Discord notification channel.
pub struct DiscordChannel {
    config: DiscordConfig,
    client: Client,
    template_engine: TemplateEngine,
}

impl EmailChannel {
    /// Create a new email channel.
    pub fn new(config: EmailConfig) -> NotifierResult<Self> {
        let creds = Credentials::new(config.username.clone(), config.password.clone());
        
        let transport = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_server)
                .map_err(|e| NotifierError::SmtpTransportBuild(e.to_string()))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_server)
        }
        .port(config.smtp_port)
        .credentials(creds)
        .pool_config(PoolConfig::new().max_size(10))
        .build();

        Ok(Self {
            config,
            transport,
            template_engine: TemplateEngine::new(),
        })
    }
}

#[async_trait]
impl NotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    async fn send(&self, alert: &Alert, template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        let subject = if let Some(template) = &self.config.subject_template {
            self.template_engine.render_template(template, template_data)?
        } else {
            format!("[Watchtower] {} Alert: {}", alert.severity.as_str().to_uppercase(), alert.rule_name)
        };

        let body = if let Some(template) = &self.config.body_template {
            self.template_engine.render_template(template, template_data)?
        } else {
            self.template_engine.render_default_email_template(alert)?
        };

        let from_mailbox = if let Some(from_name) = &self.config.from_name {
            Mailbox::new(Some(from_name.clone()), self.config.from_address.parse()?)
        } else {
            self.config.from_address.parse()?
        };

        for to_address in &self.config.to_addresses {
            let email = Message::builder()
                .from(from_mailbox.clone())
                .to(to_address.parse()?)
                .subject(&subject)
                .header(ContentType::TEXT_HTML)
                .body(body.clone())?;

            match self.transport.send(email).await {
                Ok(_) => {
                    info!("Email sent successfully to {}", to_address);
                }
                Err(e) => {
                    error!("Failed to send email to {}: {}", to_address, e);
                    return Err(NotifierError::SmtpTransport(e));
                }
            }
        }

        Ok(())
    }

    async fn test(&self) -> NotifierResult<()> {
        // Send a test email
        let test_data = HashMap::new();
        let test_alert = Alert {
            id: "test".to_string(),
            rule_name: "test_rule".to_string(),
            message: "This is a test alert".to_string(),
            severity: watchtower_engine::AlertSeverity::Info,
            program_id: solana_sdk::pubkey::Pubkey::new_unique(),
            program_name: "Test Program".to_string(),
            event_id: None,
            metadata: HashMap::new(),
            confidence: 1.0,
            suggested_actions: vec!["This is a test".to_string()],
            timestamp: chrono::Utc::now(),
            acknowledged: false,
            resolved: false,
        };

        self.send(&test_alert, &test_data).await
    }

    fn supports_batching(&self) -> bool {
        true
    }

    async fn send_batch(&self, alerts: &[Alert], template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        let subject = format!("[Watchtower] {} Alerts", alerts.len());
        let body = self.template_engine.render_batch_email_template(alerts)?;

        let from_mailbox = if let Some(from_name) = &self.config.from_name {
            Mailbox::new(Some(from_name.clone()), self.config.from_address.parse()?)
        } else {
            self.config.from_address.parse()?
        };

        for to_address in &self.config.to_addresses {
            let email = Message::builder()
                .from(from_mailbox.clone())
                .to(to_address.parse()?)
                .subject(&subject)
                .header(ContentType::TEXT_HTML)
                .body(body.clone())?;

            self.transport.send(email).await.map_err(NotifierError::SmtpTransport)?;
        }

        info!("Batch email sent with {} alerts", alerts.len());
        Ok(())
    }
}

impl TelegramChannel {
    /// Create a new Telegram channel.
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            template_engine: TemplateEngine::new(),
        }
    }
}

#[async_trait]
impl NotificationChannel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(&self, alert: &Alert, template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        let message = if let Some(template) = &self.config.message_template {
            self.template_engine.render_template(template, template_data)?
        } else {
            self.template_engine.render_default_telegram_template(alert)?
        };

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.config.bot_token);
        
        let mut payload = json!({
            "chat_id": self.config.chat_id,
            "text": message,
            "disable_web_page_preview": self.config.disable_web_page_preview,
            "disable_notification": self.config.disable_notification,
        });

        if !self.config.parse_mode.is_empty() {
            payload["parse_mode"] = json!(self.config.parse_mode);
        }

        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(NotifierError::Generic(format!("Telegram API error: {}", error_text)));
        }

        info!("Telegram message sent successfully");
        Ok(())
    }

    async fn test(&self) -> NotifierResult<()> {
        let test_data = HashMap::new();
        let test_alert = Alert {
            id: "test".to_string(),
            rule_name: "test_rule".to_string(),
            message: "This is a test alert".to_string(),
            severity: watchtower_engine::AlertSeverity::Info,
            program_id: solana_sdk::pubkey::Pubkey::new_unique(),
            program_name: "Test Program".to_string(),
            event_id: None,
            metadata: HashMap::new(),
            confidence: 1.0,
            suggested_actions: vec!["This is a test".to_string()],
            timestamp: chrono::Utc::now(),
            acknowledged: false,
            resolved: false,
        };

        self.send(&test_alert, &test_data).await
    }
}

impl SlackChannel {
    /// Create a new Slack channel.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            template_engine: TemplateEngine::new(),
        }
    }
}

#[async_trait]
impl NotificationChannel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, alert: &Alert, template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        let text = if let Some(template) = &self.config.message_template {
            self.template_engine.render_template(template, template_data)?
        } else {
            self.template_engine.render_default_slack_template(alert)?
        };

        let mut payload = json!({
            "text": text,
        });

        if let Some(channel) = &self.config.channel {
            payload["channel"] = json!(channel);
        }

        if let Some(username) = &self.config.username {
            payload["username"] = json!(username);
        }

        if let Some(icon) = &self.config.icon {
            if icon.starts_with(':') && icon.ends_with(':') {
                payload["icon_emoji"] = json!(icon);
            } else {
                payload["icon_url"] = json!(icon);
            }
        }

        // Add alert severity color
        let color = match alert.severity {
            watchtower_engine::AlertSeverity::Critical => "#ff0000",
            watchtower_engine::AlertSeverity::High => "#ff8c00",
            watchtower_engine::AlertSeverity::Medium => "#ffd700",
            watchtower_engine::AlertSeverity::Low => "#32cd32",
            watchtower_engine::AlertSeverity::Info => "#87ceeb",
        };

        payload["attachments"] = json!([{
            "color": color,
            "fields": [
                {
                    "title": "Program",
                    "value": alert.program_name,
                    "short": true
                },
                {
                    "title": "Severity", 
                    "value": alert.severity.as_str(),
                    "short": true
                },
                {
                    "title": "Confidence",
                    "value": format!("{:.1}%", alert.confidence * 100.0),
                    "short": true
                }
            ],
            "ts": alert.timestamp.timestamp()
        }]);

        let response = self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(NotifierError::Generic(format!("Slack webhook failed: {}", error_text)));
        }

        info!("Slack message sent successfully");
        Ok(())
    }

    async fn test(&self) -> NotifierResult<()> {
        let test_data = HashMap::new();
        let test_alert = Alert {
            id: "test".to_string(),
            rule_name: "test_rule".to_string(),
            message: "This is a test alert".to_string(),
            severity: watchtower_engine::AlertSeverity::Info,
            program_id: solana_sdk::pubkey::Pubkey::new_unique(),
            program_name: "Test Program".to_string(),
            event_id: None,
            metadata: HashMap::new(),
            confidence: 1.0,
            suggested_actions: vec!["This is a test".to_string()],
            timestamp: chrono::Utc::now(),
            acknowledged: false,
            resolved: false,
        };

        self.send(&test_alert, &test_data).await
    }
}

impl DiscordChannel {
    /// Create a new Discord channel.
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            template_engine: TemplateEngine::new(),
        }
    }
}

#[async_trait]
impl NotificationChannel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn send(&self, alert: &Alert, template_data: &HashMap<String, Value>) -> NotifierResult<()> {
        let content = if let Some(template) = &self.config.message_template {
            self.template_engine.render_template(template, template_data)?
        } else {
            self.template_engine.render_default_discord_template(alert)?
        };

        let mut payload = json!({
            "content": content,
        });

        if let Some(username) = &self.config.username {
            payload["username"] = json!(username);
        }

        if let Some(avatar_url) = &self.config.avatar_url {
            payload["avatar_url"] = json!(avatar_url);
        }

        if self.config.use_embeds {
            let color = match alert.severity {
                watchtower_engine::AlertSeverity::Critical => 0xff0000,
                watchtower_engine::AlertSeverity::High => 0xff8c00,
                watchtower_engine::AlertSeverity::Medium => 0xffd700,
                watchtower_engine::AlertSeverity::Low => 0x32cd32,
                watchtower_engine::AlertSeverity::Info => 0x87ceeb,
            };

            payload["embeds"] = json!([{
                "title": format!("{} Alert", alert.severity.as_str().to_uppercase()),
                "description": alert.message,
                "color": color,
                "fields": [
                    {
                        "name": "Rule",
                        "value": alert.rule_name,
                        "inline": true
                    },
                    {
                        "name": "Program",
                        "value": alert.program_name,
                        "inline": true
                    },
                    {
                        "name": "Confidence",
                        "value": format!("{:.1}%", alert.confidence * 100.0),
                        "inline": true
                    }
                ],
                "timestamp": alert.timestamp.to_rfc3339()
            }]);
        }

        let response = self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(NotifierError::Generic(format!("Discord webhook failed: {}", error_text)));
        }

        info!("Discord message sent successfully");
        Ok(())
    }

    async fn test(&self) -> NotifierResult<()> {
        let test_data = HashMap::new();
        let test_alert = Alert {
            id: "test".to_string(),
            rule_name: "test_rule".to_string(),
            message: "This is a test alert".to_string(),
            severity: watchtower_engine::AlertSeverity::Info,
            program_id: solana_sdk::pubkey::Pubkey::new_unique(),
            program_name: "Test Program".to_string(),
            event_id: None,
            metadata: HashMap::new(),
            confidence: 1.0,
            suggested_actions: vec!["This is a test".to_string()],
            timestamp: chrono::Utc::now(),
            acknowledged: false,
            resolved: false,
        };

        self.send(&test_alert, &test_data).await
    }
} 